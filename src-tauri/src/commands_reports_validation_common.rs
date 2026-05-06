use serde_json::{Map, Value};

const CSCV_RANK_FRAC_MIN: f64 = 0.55;
const ACTIVE_PROFILE_KEYS: [&str; 10] = [
    "asr", "vcc", "vemr", "rocb", "mkcs", "kqsr", "carm", "kvdf", "cdmc", "vcet",
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ExportValidationProvenance {
    pub stage_sequence: String,
    pub score_channel: String,
    pub passed_vs_approved_contract: String,
    pub probability_metric_source: String,
    pub track_record_source: String,
    pub cscv_coverage_state: String,
    pub cscv_candidate_stat_coverage: String,
    pub cscv_selection_state: String,
    pub cross_summary_scope: String,
    pub cross_mode: String,
    pub cross_summary_mode: String,
    pub regime_summary_family: String,
    pub regime_summary_role: String,
    pub regime_summary_semantics: String,
    pub regime_summary_descriptive_only: Option<bool>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ExportValidationEvidencePack {
    pub schema_version: String,
    pub version: Option<u64>,
    pub workflow: String,
    pub status: String,
    pub status_reason: String,
    pub generated_by: String,
    pub requested_backend: String,
    pub realized_backend: String,
    pub route_label: String,
    pub residency_tier: String,
    pub fallback_reason: String,
    pub candidate_source: String,
    pub requested_candidates: usize,
    pub evaluated_candidates: usize,
    pub retained_candidates: usize,
    pub survived_hard_gates: usize,
    pub approved_candidates: usize,
    pub optimization_trials: Option<usize>,
    pub verdict_summary: String,
    pub verdict_recommendation: String,
    pub module_statuses: String,
    pub metric_summaries: String,
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
    pub shortlist_count: usize,
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
    pub monte_carlo_mode: String,
    pub monte_carlo_prob_pnl_positive: Option<f64>,
    pub monte_carlo_pnl_p05: Option<f64>,
    pub monte_carlo_pnl_p50: Option<f64>,
    pub monte_carlo_max_dd_p95: Option<f64>,
    pub monte_carlo_stationary_mean_block_len: Option<usize>,
    pub monte_carlo_stationary_pnl_p05: Option<f64>,
    pub monte_carlo_stationary_max_dd_p95: Option<f64>,
    pub monte_carlo_sample_count: Option<usize>,
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
    pub requested_backend: String,
    pub validation_acceleration_status: String,
    pub validation_residency_tier: String,
    pub backend_used: String,
    pub backend_route: String,
    pub backend_fallback_reason: String,
    pub actual_fallback_summary: String,
    pub actual_route_note_summary: String,
    pub actual_aux_tail_summary: String,
    pub actual_disabled_module_summary: String,
    pub actual_skipped_module_summary: String,
    pub actual_cross_omitted_summary: String,
    pub expected_route_summary: String,
    pub expected_cross_summary: String,
    pub expected_module_summary: String,
    pub expected_category_summary: String,
    pub expected_fallback_summary: String,
    pub expected_route_note_summary: String,
    pub expected_disabled_module_summary: String,
    pub expected_cross_omitted_summary: String,
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
    #[allow(dead_code)]
    pub validation_provenance: Option<ExportValidationProvenance>,
    #[allow(dead_code)]
    pub validation_evidence_pack: Option<ExportValidationEvidencePack>,
    pub spa: Option<ExportSpaSummary>,
    pub reopt_walk_forward: Option<ExportReoptSummary>,
    pub cscv_pbo: Option<ExportCscvSummary>,
    pub cross_ticker: Option<ExportCrossSummary>,
    pub ranking_stability: Option<ExportRankingStabilitySummary>,
    pub all_rows: Vec<ExportCandidateRow>,
    pub approved_rows: Vec<ExportCandidateRow>,
    pub shortlist_rows: Vec<ExportCandidateRow>,
}

#[derive(Clone, Debug, Default)]
struct ExpectedExecutionSummary {
    route_summary: String,
    cross_summary: String,
    module_summary: String,
    category_summary: String,
    fallback_summary: String,
    route_note_summary: String,
    disabled_module_summary: String,
    cross_omitted_summary: String,
    warning_summary: String,
}

fn normalize_text(value: Option<&str>) -> String {
    value.unwrap_or("").trim().to_string()
}

fn first_nonempty_text(v: &Value, ptrs: &[&str]) -> String {
    ptrs.iter()
        .find_map(|ptr| {
            v.pointer(ptr)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_default()
}

fn first_bool(v: &Value, ptrs: &[&str]) -> Option<bool> {
    ptrs.iter()
        .find_map(|ptr| v.pointer(ptr).and_then(Value::as_bool))
}

fn first_joined_text_array(v: &Value, ptrs: &[&str], sep: &str) -> String {
    ptrs.iter()
        .find_map(|ptr| {
            v.pointer(ptr).and_then(Value::as_array).map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|text| !text.is_empty())
                    .collect::<Vec<_>>()
                    .join(sep)
            })
        })
        .filter(|joined| !joined.is_empty())
        .unwrap_or_default()
}

fn opt_f64(v: &Value, ptr: &str) -> Option<f64> {
    v.pointer(ptr)
        .and_then(|x| x.as_f64())
        .filter(|x| x.is_finite())
}

fn opt_u64(v: &Value, ptr: &str) -> Option<u64> {
    v.pointer(ptr).and_then(|x| x.as_u64())
}

fn first_opt_f64(v: &Value, ptrs: &[&str]) -> Option<f64> {
    ptrs.iter().find_map(|ptr| opt_f64(v, ptr))
}

fn first_opt_u64(v: &Value, ptrs: &[&str]) -> Option<u64> {
    ptrs.iter().find_map(|ptr| opt_u64(v, ptr))
}

fn active_monte_carlo_request(validation_req: &Value) -> Option<&Value> {
    validation_req
        .get("trade_bootstrap")
        .filter(|value| !value.is_null())
        .or_else(|| {
            validation_req
                .get("monte_carlo")
                .filter(|value| !value.is_null())
        })
}

fn validation_module_enabled(validation_req: &Value, module: &str) -> bool {
    if module == "trade_bootstrap" {
        active_monte_carlo_request(validation_req).is_some()
    } else {
        validation_req
            .get(module)
            .is_some_and(|value| !value.is_null())
    }
}

fn monte_carlo_request_has_mode(request: &Value, mode: &str) -> bool {
    request
        .get("modes")
        .and_then(Value::as_array)
        .is_some_and(|modes| modes.iter().any(|value| value.as_str() == Some(mode)))
}

fn bootstrap_iid_metric(cand: &Value, legacy_ptr: &str, mode_field: &str) -> Option<f64> {
    let monte_carlo_ptr = format!("/monte_carlo/modes/trade_iid/{mode_field}");
    first_opt_f64(cand, &[legacy_ptr, monte_carlo_ptr.as_str()])
}

fn bootstrap_stationary_metric(cand: &Value, legacy_ptr: &str, mode_field: &str) -> Option<f64> {
    let monte_carlo_ptr = format!("/monte_carlo/modes/trade_stationary/{mode_field}");
    first_opt_f64(cand, &[legacy_ptr, monte_carlo_ptr.as_str()])
}

fn bootstrap_stationary_block_len(cand: &Value) -> Option<usize> {
    first_opt_u64(
        cand,
        &[
            "/trade_bootstrap/stationary_mean_block_len",
            "/monte_carlo/modes/trade_stationary/stationary_mean_block_len",
        ],
    )
    .map(|v| v as usize)
}

fn monte_carlo_sample_count(cand: &Value) -> Option<usize> {
    first_opt_u64(
        cand,
        &[
            "/monte_carlo/simulations",
            "/monte_carlo/sample_count",
            "/monte_carlo/modes/trade_iid/simulations",
            "/monte_carlo/modes/trade_iid/sample_count",
            "/trade_bootstrap/simulations",
            "/trade_bootstrap/sample_count",
        ],
    )
    .map(|v| v as usize)
}

fn monte_carlo_mode_label(
    iid_prob: Option<f64>,
    iid_p05: Option<f64>,
    iid_dd95: Option<f64>,
    stat_block: Option<usize>,
    stat_p05: Option<f64>,
    stat_dd95: Option<f64>,
) -> String {
    if stat_block.is_some() || stat_p05.is_some() || stat_dd95.is_some() {
        "compare_iid_stationary".to_string()
    } else if iid_prob.is_some() || iid_p05.is_some() || iid_dd95.is_some() {
        "iid_only".to_string()
    } else {
        String::new()
    }
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
    let Some(regime) = cand
        .get("regime_slice_robustness")
        .and_then(Value::as_object)
    else {
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
            let pnl = bucket
                .get("pnl")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite());
            let sharpe = bucket
                .get("sharpe")
                .and_then(Value::as_f64)
                .filter(|v| v.is_finite());
            Some(ExportRegimeBucketRow {
                rank: 0,
                approval_label: String::new(),
                params_summary: String::new(),
                label: (*label).to_string(),
                bars: bucket
                    .get("bars")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize),
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
                trades: bucket
                    .get("trades")
                    .and_then(Value::as_u64)
                    .map(|v| v as usize),
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
    let Some(request) = validation_req.and_then(active_monte_carlo_request) else {
        return "-".to_string();
    };
    if request
        .get("stationary_mean_block_len")
        .and_then(Value::as_u64)
        == Some(0)
        || (request.get("modes").is_some()
            && monte_carlo_request_has_mode(request, "trade_iid")
            && !monte_carlo_request_has_mode(request, "trade_stationary"))
    {
        return "iid-only closed-trade bootstrap".to_string();
    }
    let stat_block = request
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
    let passed = verdict.get("passed").and_then(Value::as_bool)?;
    let approved = verdict.get("approved").and_then(Value::as_bool)?;
    Some(ExportRunVerdict {
        passed,
        approved,
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

fn parse_validation_provenance(validation_res: &Value) -> Option<ExportValidationProvenance> {
    let provenance = validation_res
        .get("validation_provenance")
        .or_else(|| validation_res.get("provenance"))?;

    let parsed = ExportValidationProvenance {
        stage_sequence: first_joined_text_array(
            provenance,
            &["/stage_sequence", "/stages", "/validation_stage_sequence"],
            " -> ",
        ),
        score_channel: first_nonempty_text(
            provenance,
            &["/score_channel", "/score_channel/name", "/score/name"],
        ),
        passed_vs_approved_contract: first_nonempty_text(
            provenance,
            &[
                "/passed_vs_approved_contract",
                "/passed_approved_contract",
                "/run_verdict/passed_vs_approved_contract",
                "/run_verdict/contract",
            ],
        ),
        probability_metric_source: first_nonempty_text(
            provenance,
            &[
                "/probability_metric_source",
                "/probability_metric/source",
                "/probability/source",
                "/score_channel/source",
            ],
        ),
        track_record_source: first_nonempty_text(
            provenance,
            &["/track_record_source", "/track_record/source"],
        ),
        cscv_coverage_state: first_nonempty_text(
            provenance,
            &[
                "/cscv_coverage_state",
                "/cscv/coverage_state",
                "/cscv/state/coverage",
            ],
        ),
        cscv_candidate_stat_coverage: first_nonempty_text(
            provenance,
            &[
                "/cscv_candidate_stat_coverage",
                "/cscv/candidate_stat_coverage",
                "/cscv/state/candidate_stat_coverage",
            ],
        ),
        cscv_selection_state: first_nonempty_text(
            provenance,
            &[
                "/cscv_selection_state",
                "/cscv/selection_state",
                "/cscv/state/selection",
            ],
        ),
        cross_summary_scope: first_nonempty_text(
            provenance,
            &[
                "/cross_summary_scope",
                "/cross_summary/scope",
                "/cross/scope",
            ],
        ),
        cross_mode: first_nonempty_text(provenance, &["/cross_mode", "/cross/mode_kind"]),
        cross_summary_mode: first_nonempty_text(
            provenance,
            &["/cross_summary_mode", "/cross_summary/mode", "/cross/mode"],
        ),
        regime_summary_family: first_nonempty_text(
            provenance,
            &[
                "/regime_summary_family",
                "/regime_summary/family",
                "/regime_summary/tag",
            ],
        ),
        regime_summary_role: first_nonempty_text(
            provenance,
            &[
                "/regime_summary_role",
                "/regime_summary/role",
                "/regime_summary/channel",
            ],
        ),
        regime_summary_semantics: first_nonempty_text(
            provenance,
            &[
                "/regime_summary_semantics",
                "/regime_summary/semantics",
                "/regime_summary/contract",
            ],
        ),
        regime_summary_descriptive_only: first_bool(
            provenance,
            &[
                "/regime_summary_descriptive_only",
                "/regime_summary/descriptive_only",
                "/regime_summary/is_descriptive_only",
            ],
        ),
    };

    if parsed.stage_sequence.is_empty()
        && parsed.score_channel.is_empty()
        && parsed.passed_vs_approved_contract.is_empty()
        && parsed.probability_metric_source.is_empty()
        && parsed.track_record_source.is_empty()
        && parsed.cscv_coverage_state.is_empty()
        && parsed.cscv_candidate_stat_coverage.is_empty()
        && parsed.cscv_selection_state.is_empty()
        && parsed.cross_summary_scope.is_empty()
        && parsed.cross_mode.is_empty()
        && parsed.cross_summary_mode.is_empty()
        && parsed.regime_summary_family.is_empty()
        && parsed.regime_summary_role.is_empty()
        && parsed.regime_summary_semantics.is_empty()
        && parsed.regime_summary_descriptive_only.is_none()
    {
        None
    } else {
        Some(parsed)
    }
}

fn parse_validation_evidence_pack(validation_res: &Value) -> Option<ExportValidationEvidencePack> {
    let pack = validation_res
        .get("validation_evidence_pack")
        .or_else(|| validation_res.get("validationEvidencePack"))?;
    let route = pack.get("route").unwrap_or(&Value::Null);
    let universe = pack
        .get("candidate_universe")
        .or_else(|| pack.get("candidateUniverse"))
        .unwrap_or(&Value::Null);
    let verdict = pack.get("verdict").unwrap_or(&Value::Null);
    let module_statuses = pack
        .get("modules")
        .and_then(Value::as_array)
        .map(|modules| {
            modules
                .iter()
                .filter_map(|module| {
                    let name = first_nonempty_text(module, &["/module", "/id"]);
                    if name.is_empty() {
                        return None;
                    }
                    let availability = first_nonempty_text(module, &["/availability"]);
                    let summary = first_nonempty_text(module, &["/summary"]);
                    Some(if summary.is_empty() {
                        format!("{name}={availability}")
                    } else {
                        format!("{name}={availability}:{summary}")
                    })
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();
    let metric_summaries = pack
        .get("metrics")
        .and_then(Value::as_array)
        .map(|metrics| {
            metrics
                .iter()
                .filter_map(|metric| {
                    let name = first_nonempty_text(metric, &["/metric"]);
                    if name.is_empty() {
                        return None;
                    }
                    let availability = first_nonempty_text(metric, &["/availability"]);
                    let value = metric
                        .get("value")
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite())
                        .map(|value| format!("{value:.6}"));
                    let threshold = metric
                        .get("threshold")
                        .and_then(Value::as_f64)
                        .filter(|value| value.is_finite())
                        .map(|value| format!("{value:.6}"));
                    Some(match (value, threshold) {
                        (Some(value), Some(threshold)) => {
                            format!("{name}={value} threshold={threshold}")
                        }
                        (Some(value), None) => format!("{name}={value}"),
                        (None, _) => format!("{name}={availability}"),
                    })
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_default();

    Some(ExportValidationEvidencePack {
        schema_version: first_nonempty_text(pack, &["/schema_version", "/schemaVersion"]),
        version: pack.get("version").and_then(Value::as_u64),
        workflow: first_nonempty_text(pack, &["/workflow"]),
        status: first_nonempty_text(pack, &["/status"]),
        status_reason: first_nonempty_text(pack, &["/status_reason", "/statusReason"]),
        generated_by: first_nonempty_text(pack, &["/generated_by", "/generatedBy"]),
        requested_backend: first_nonempty_text(route, &["/requested_backend", "/requestedBackend"]),
        realized_backend: first_nonempty_text(route, &["/realized_backend", "/realizedBackend"]),
        route_label: first_nonempty_text(route, &["/route_label", "/routeLabel"]),
        residency_tier: first_nonempty_text(route, &["/residency_tier", "/residencyTier"]),
        fallback_reason: first_nonempty_text(route, &["/fallback_reason", "/fallbackReason"]),
        candidate_source: first_nonempty_text(universe, &["/source"]),
        requested_candidates: universe
            .get("requested_candidates")
            .or_else(|| universe.get("requestedCandidates"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        evaluated_candidates: universe
            .get("evaluated_candidates")
            .or_else(|| universe.get("evaluatedCandidates"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        retained_candidates: universe
            .get("retained_candidates")
            .or_else(|| universe.get("retainedCandidates"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        survived_hard_gates: universe
            .get("survived_hard_gates")
            .or_else(|| universe.get("survivedHardGates"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        approved_candidates: universe
            .get("approved_candidates")
            .or_else(|| universe.get("approvedCandidates"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        optimization_trials: universe
            .get("optimization_trials")
            .or_else(|| universe.get("optimizationTrials"))
            .and_then(Value::as_u64)
            .map(|value| value as usize),
        verdict_summary: first_nonempty_text(verdict, &["/summary"]),
        verdict_recommendation: first_nonempty_text(
            verdict,
            &[
                "/recommendation",
                "/final_recommendation",
                "/finalRecommendation",
            ],
        ),
        module_statuses,
        metric_summaries,
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
    let candidates = validation_res
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
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
    let shortlist_count = candidates
        .iter()
        .filter(|cand| {
            cand.get("robust_pareto")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && cand
                    .get("cross_ticker_pareto")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        })
        .count();
    Some(ExportCrossSummary {
        dataset_count: datasets.len(),
        mode: normalize_text(cross.get("mode").and_then(Value::as_str)),
        dataset_labels: labels,
        shortlist_count,
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

fn is_composite_double_like_strategy(strategy_kind: &str) -> bool {
    matches!(
        strategy_kind.trim(),
        "CompositeTrend" | "RegimeHybrid" | "BreakoutMomentum"
    )
}

fn normalize_boolish_json(value: Option<&Value>) -> bool {
    match value {
        Some(value) if value.is_boolean() => value.as_bool().unwrap_or(false),
        Some(value) => {
            let text = normalize_text(value.as_str()).to_ascii_lowercase();
            matches!(text.as_str(), "true" | "1" | "yes" | "on")
        }
        None => false,
    }
}

fn resolve_composite_double_like_validation_target(
    strategy_kind: &str,
    validation_req: &Value,
    backend: &str,
) -> Option<(&'static str, bool)> {
    if !is_composite_double_like_strategy(strategy_kind) {
        return None;
    }

    let strategy = validation_req
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("strategy"));
    let trend_use_adx =
        normalize_boolish_json(strategy.and_then(|row| row.get("trend_quality_use_adx")));
    let breakout_use_adx =
        normalize_boolish_json(strategy.and_then(|row| row.get("breakout_use_adx")));

    if backend == "CpuOnly" {
        return Some(("CpuOnly", false));
    }
    if matches!(backend, "Auto" | "GpuKernel")
        && strategy_kind == "CompositeTrend"
        && !trend_use_adx
    {
        return Some(("CpuOnly", backend != "Auto"));
    }
    if backend == "GpuHybridKernel" {
        return Some(("GpuHybridKernel", false));
    }
    if strategy_kind == "RegimeHybrid" {
        return Some(("GpuHybridKernel", false));
    }
    if strategy_kind == "CompositeTrend" && trend_use_adx {
        return Some(("GpuHybridKernel", false));
    }
    if strategy_kind == "BreakoutMomentum" && breakout_use_adx {
        return Some(("GpuHybridKernel", false));
    }
    Some(("GpuKernel", false))
}

fn composite_double_like_expected_route_text(
    strategy_kind: &str,
    validation_req: &Value,
    backend: &str,
    mode: &str,
) -> Option<(String, String)> {
    let (target_backend, cpu_fallback) =
        resolve_composite_double_like_validation_target(strategy_kind, validation_req, backend)?;

    let route_descriptor = match target_backend {
        "GpuHybridKernel" => "GPU hybrid main route",
        "GpuKernel" => "GPU kernel main route",
        _ => "CPU route",
    };
    let module_route_descriptor = match target_backend {
        "GpuHybridKernel" => "Expected GPU hybrid route",
        "GpuKernel" => "Expected GPU kernel route",
        _ => "CPU route",
    };
    let suffix = if mode == "auto" && target_backend != "CpuOnly" {
        " when available"
    } else {
        ""
    };

    if mode == "requested" && cpu_fallback {
        return Some((
            format!(
                "{backend} requested | expected CpuOnly fallback for fixed-candidate validation ({strategy_kind})"
            ),
            format!("Expected CPU fallback route ({strategy_kind})"),
        ));
    }

    let route_summary = if mode == "auto" {
        format!("Auto requested | expected {route_descriptor} ({strategy_kind}){suffix}")
    } else {
        format!("{backend} requested | expected {route_descriptor} ({strategy_kind})")
    };
    let module_summary_prefix = if mode == "auto" {
        format!("{module_route_descriptor} ({strategy_kind}){suffix}")
    } else {
        format!("{module_route_descriptor} ({strategy_kind}){suffix}")
    };

    Some((route_summary, module_summary_prefix))
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
    if let Some(bootstrap) = active_monte_carlo_request(validation_req) {
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

fn is_composite_gpu_native_advanced_strategy(strategy_kind: &str) -> bool {
    matches!(
        strategy_kind,
        "Asr" | "Vcc" | "Vemr" | "Rocb" | "Vcet" | "Mkcs" | "Kqsr" | "Carm" | "Kvdf" | "Cdmc"
    )
}

fn describe_composite_gpu_native_optional_modules(validation_req: &Value) -> Vec<String> {
    let mut modules = Vec::new();
    if validation_req.get("perturb").is_some_and(|v| !v.is_null()) {
        modules.push("perturb".to_string());
    }
    if let Some(bootstrap) = active_monte_carlo_request(validation_req) {
        if bootstrap
            .get("stationary_mean_block_len")
            .and_then(Value::as_u64)
            == Some(0)
        {
            modules.push("trade bootstrap iid-only".to_string());
            return modules;
        }
        let stat_block = bootstrap
            .get("stationary_mean_block_len")
            .and_then(Value::as_u64)
            .map(|v| v.to_string())
            .unwrap_or_else(|| "auto".to_string());
        modules.push(format!(
            "trade bootstrap compare (iid + stationary, stat block={stat_block})"
        ));
    }
    modules
}

fn describe_disabled_modules(validation_req: &Value) -> Vec<String> {
    let mut disabled = Vec::new();
    for module in [
        "walk_forward",
        "reopt_walk_forward",
        "cscv_pbo",
        "spa",
        "perturb",
        "trade_bootstrap",
        "cross_ticker",
    ] {
        if !validation_module_enabled(validation_req, module) {
            disabled.push(module.to_string());
        }
    }
    disabled
}

fn describe_cross_omitted_modules(strategy_kind: &str, validation_req: &Value) -> Vec<String> {
    let Some(cross) = validation_req
        .get("cross_ticker")
        .and_then(Value::as_object)
    else {
        return Vec::new();
    };
    let mode = cross
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("default")
        .trim();
    if mode == "full_stack" {
        return Vec::new();
    }
    if is_double_like_strategy(strategy_kind) && mode == "preserved_finalists" {
        return vec![
            "walk_forward".to_string(),
            "reopt_walk_forward".to_string(),
            "spa".to_string(),
            "perturb".to_string(),
            "trade_bootstrap".to_string(),
        ];
    }
    if !is_double_like_strategy(strategy_kind) {
        if mode == "preserved_finalists" {
            return vec![
                "walk_forward".to_string(),
                "reopt_walk_forward".to_string(),
                "spa".to_string(),
                "perturb".to_string(),
                "trade_bootstrap".to_string(),
            ];
        }
        return vec![
            "walk_forward".to_string(),
            "reopt_walk_forward".to_string(),
            "spa".to_string(),
            "perturb".to_string(),
            "trade_bootstrap".to_string(),
            "cscv_pbo".to_string(),
            "hard gates".to_string(),
        ];
    }
    Vec::new()
}

fn summary_or_dash(items: Vec<String>, delimiter: &str) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(delimiter)
    }
}

fn split_actual_route_reason(
    backend_route: &str,
    backend_fallback_reason: &str,
) -> (String, String) {
    if backend_fallback_reason.trim().is_empty() {
        return ("-".to_string(), "-".to_string());
    }
    if backend_route.contains("->CpuOnly") {
        (backend_fallback_reason.trim().to_string(), "-".to_string())
    } else {
        ("-".to_string(), backend_fallback_reason.trim().to_string())
    }
}

fn build_validation_acceleration_status(
    backend_used: &str,
    backend_route: &str,
    requested_backend: &str,
    backend_fallback_reason: &str,
    has_cpu_followup: bool,
) -> String {
    let used = backend_used.trim().to_ascii_lowercase();
    let route = backend_route.trim();
    let route_lower = route.to_ascii_lowercase();
    let requested = requested_backend.trim().to_ascii_lowercase();
    let fallback_reason = backend_fallback_reason.trim();

    if used == "gpu" && !route_lower.contains("->cpuonly") {
        if has_cpu_followup {
            "GPU validation completed with CPU follow-up checks.".to_string()
        } else {
            "GPU validation completed.".to_string()
        }
    } else if route_lower.contains("->cpuonly")
        || (used == "cpu" && !requested.is_empty() && requested != "cpuonly")
    {
        if fallback_reason.is_empty() {
            "CPU validation used because GPU validation was unavailable.".to_string()
        } else {
            format!("GPU validation unavailable: {fallback_reason}")
        }
    } else if used == "cpu" {
        "CPU validation completed.".to_string()
    } else {
        "Validation acceleration status unavailable.".to_string()
    }
}

fn enabled_execution_category_modules(validation_req: &Value) -> Vec<&'static str> {
    let mut modules = vec!["base reruns", "hard gates"];
    if validation_req
        .get("walk_forward")
        .is_some_and(|v| !v.is_null())
    {
        modules.push("walk_forward");
    }
    if validation_req
        .get("reopt_walk_forward")
        .is_some_and(|v| !v.is_null())
    {
        modules.push("reopt_walk_forward");
    }
    if validation_req.get("cscv_pbo").is_some_and(|v| !v.is_null()) {
        modules.push("cscv_pbo");
    }
    if validation_req.get("spa").is_some_and(|v| !v.is_null()) {
        modules.push("spa");
    }
    if validation_req.get("perturb").is_some_and(|v| !v.is_null()) {
        modules.push("perturb");
    }
    if validation_module_enabled(validation_req, "trade_bootstrap") {
        modules.push("trade_bootstrap");
    }
    modules
}

fn format_execution_category_summary(groups: Vec<(&'static str, Vec<&'static str>)>) -> String {
    let parts: Vec<String> = groups
        .into_iter()
        .filter(|(_, labels)| !labels.is_empty())
        .map(|(category, labels)| format!("{category}: {}", labels.join(" + ")))
        .collect();
    if parts.is_empty() {
        "none".to_string()
    } else {
        parts.join(" | ")
    }
}

fn build_expected_execution_category_summary(
    strategy_kind: &str,
    validation_req: &Value,
    backend: &str,
) -> String {
    let modules = enabled_execution_category_modules(validation_req);
    if is_double_like_strategy(strategy_kind) {
        let composite_double_like_target =
            resolve_composite_double_like_validation_target(strategy_kind, validation_req, backend)
                .map(|(target, _)| target);
        if backend == "CpuOnly"
            || backend == "GpuOnly"
            || composite_double_like_target.is_some_and(|target| target == "CpuOnly")
        {
            return format_execution_category_summary(vec![("CPU-only", modules)]);
        }
        let mut gpu_native = vec!["base reruns", "hard gates"];
        if validation_req
            .get("walk_forward")
            .is_some_and(|v| !v.is_null())
        {
            gpu_native.push("walk_forward");
        }
        if validation_req
            .get("reopt_walk_forward")
            .is_some_and(|v| !v.is_null())
        {
            gpu_native.push("reopt_walk_forward");
        }
        if validation_req.get("cscv_pbo").is_some_and(|v| !v.is_null()) {
            gpu_native.push("cscv_pbo");
        }
        if validation_req.get("spa").is_some_and(|v| !v.is_null()) {
            gpu_native.push("spa");
        }
        let mut gpu_tail = Vec::new();
        if validation_req.get("perturb").is_some_and(|v| !v.is_null()) {
            gpu_tail.push("perturb");
        }
        if validation_module_enabled(validation_req, "trade_bootstrap") {
            gpu_tail.push("trade_bootstrap");
        }
        return format_execution_category_summary(vec![
            ("GPU-native", gpu_native),
            ("GPU main + CPU tail", gpu_tail),
        ]);
    }

    let cpu_preferred = describe_composite_auto_cpu_preference(strategy_kind, backend).is_some();
    let cpu_fallback = strategy_kind == "CompositeMo";
    if backend == "CpuOnly" || cpu_preferred || cpu_fallback {
        return format_execution_category_summary(vec![("CPU-only", modules)]);
    }
    let mut gpu_native = Vec::new();
    let mut gpu_tail = Vec::new();
    let gpu_native_advanced = is_composite_gpu_native_advanced_strategy(strategy_kind);

    let base_category = &mut gpu_native;
    base_category.push("base reruns");
    base_category.push("hard gates");

    if validation_req
        .get("walk_forward")
        .is_some_and(|v| !v.is_null())
    {
        gpu_native.push("walk_forward");
    }
    if validation_req
        .get("reopt_walk_forward")
        .is_some_and(|v| !v.is_null())
    {
        gpu_native.push("reopt_walk_forward");
    }
    if validation_req.get("cscv_pbo").is_some_and(|v| !v.is_null()) {
        gpu_native.push("cscv_pbo");
    }
    if validation_req.get("spa").is_some_and(|v| !v.is_null()) {
        gpu_native.push("spa");
    }
    if validation_req.get("perturb").is_some_and(|v| !v.is_null()) {
        if gpu_native_advanced {
            gpu_native.push("perturb");
        } else {
            gpu_tail.push("perturb");
        }
    }
    if validation_module_enabled(validation_req, "trade_bootstrap") {
        if gpu_native_advanced {
            gpu_native.push("trade_bootstrap");
        } else {
            gpu_tail.push("trade_bootstrap");
        }
    }
    format_execution_category_summary(vec![
        ("GPU-native", gpu_native),
        ("GPU main + CPU tail", gpu_tail),
    ])
}

fn describe_composite_route_detail(strategy_kind: &str, backend: &str) -> Option<String> {
    if backend == "CpuOnly" {
        return None;
    }
    match strategy_kind {
        "CompositeCpu" => Some(
            "Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU."
                .to_string(),
        ),
        "Asr" => Some(
            "ASR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Vcc" => Some(
            "VCC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route while keeping strict validation parity."
                .to_string(),
        ),
        "Vemr" => Some(
            "VEMR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Rocb" => Some(
            "ROCB GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Vcet" => Some(
            "VCET GPU validation now supports the current fixed-candidate validation module stack on the GpuKernel route, including walk-forward, reopt walk-forward, CSCV/PBO, SPA, perturb, and trade bootstrap."
                .to_string(),
        ),
        "Mkcs" => Some(
            "MKCS GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "CompositeMo" => Some(
            "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker)."
                .to_string(),
        ),
        "Kqsr" => Some(
            "KQSR GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Kvdf" => Some(
            "KVDF GPU validation currently keeps the GPU metric route, but current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Carm" => Some(
            "CARM GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route. Current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads."
                .to_string(),
        ),
        "Cdmc" => Some(
            "CDMC GPU validation now runs the current fixed-candidate validation module stack on the GpuKernel route. Fresh advanced-stack benches show GPU faster than CPU on the measured 50k/8 and 100k/32 workloads."
                .to_string(),
        ),
        _ => None,
    }
}

fn describe_composite_auto_cpu_preference(strategy_kind: &str, backend: &str) -> Option<String> {
    if backend != "Auto" {
        return None;
    }
    match strategy_kind {
        "Asr" => Some(
            "Auto composite validation currently prefers CPU for ASR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Vcc" => None,
        "Vemr" => Some(
            "Auto composite validation currently prefers CPU for VEMR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Rocb" => Some(
            "Auto composite validation currently prefers CPU for ROCB because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Mkcs" => Some(
            "Auto composite validation currently prefers CPU for MKCS because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Kqsr" => Some(
            "Auto composite validation currently prefers CPU for KQSR because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Kvdf" => Some(
            "Auto composite validation currently prefers CPU for KVDF because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 workload."
                .to_string(),
        ),
        "Carm" => Some(
            "Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads."
                .to_string(),
        ),
        _ => None,
    }
}

fn composite_profile_label_for_expected_summary(strategy_kind: &str) -> &'static str {
    match strategy_kind {
        "CompositeCpu" => "base composite",
        "Asr" => "ASR",
        "Vcc" => "VCC",
        "Vemr" => "VEMR",
        "Rocb" => "ROCB",
        "Vcet" => "VCET",
        "Mkcs" => "MKCS",
        "Kqsr" => "KQSR",
        "Carm" => "CARM",
        "Kvdf" => "KVDF",
        "Cdmc" => "CDMC",
        "CompositeMo" => "CompositeMo",
        _ => "unknown composite",
    }
}

fn build_expected_execution_summary(
    strategy_kind: &str,
    validation_req: Option<&Value>,
) -> ExpectedExecutionSummary {
    let Some(validation_req) = validation_req else {
        return ExpectedExecutionSummary {
            route_summary: "validation request unavailable".to_string(),
            cross_summary: "not available".to_string(),
            module_summary: "-".to_string(),
            category_summary: "-".to_string(),
            fallback_summary: "-".to_string(),
            route_note_summary: "-".to_string(),
            disabled_module_summary: "-".to_string(),
            cross_omitted_summary: "-".to_string(),
            warning_summary:
                "Artifact does not include validation.req; expected execution preview unavailable."
                    .to_string(),
        };
    };
    let backend = validation_req
        .pointer("/backend/backend")
        .and_then(Value::as_str)
        .unwrap_or("CpuOnly")
        .trim();
    let enabled_main = describe_enabled_modules(validation_req);
    let composite_gpu_native_advanced = !is_double_like_strategy(strategy_kind)
        && is_composite_gpu_native_advanced_strategy(strategy_kind);
    let gpu_native_optional_modules = if composite_gpu_native_advanced {
        describe_composite_gpu_native_optional_modules(validation_req)
    } else {
        Vec::new()
    };
    let aux_cpu = if composite_gpu_native_advanced {
        Vec::new()
    } else {
        describe_aux_cpu_tails(validation_req)
    };
    let gpu_native_optional_module_suffix = if gpu_native_optional_modules.is_empty() {
        String::new()
    } else {
        format!(" + {}", gpu_native_optional_modules.join(" + "))
    };
    let disabled_module_summary = summary_or_dash(describe_disabled_modules(validation_req), " + ");
    let cross_omitted_summary = summary_or_dash(
        describe_cross_omitted_modules(strategy_kind, validation_req),
        " + ",
    );
    let category_summary =
        build_expected_execution_category_summary(strategy_kind, validation_req, backend);

    let route_summary: String;
    let module_summary: String;
    let mut warning_summary = "none".to_string();
    let mut fallback_summary = "-".to_string();
    let mut route_note_summary = "-".to_string();
    let is_double_like = is_double_like_strategy(strategy_kind);
    let composite_double_like_target =
        resolve_composite_double_like_validation_target(strategy_kind, validation_req, backend);
    let composite_double_like_route = composite_double_like_expected_route_text(
        strategy_kind,
        validation_req,
        backend,
        "requested",
    );

    if is_double_like {
        match backend {
            "CpuOnly" => {
                route_summary = composite_double_like_route
                    .as_ref()
                    .map(|(route, _)| route.clone())
                    .unwrap_or_else(|| {
                        "CpuOnly requested | CPU validation route expected".to_string()
                    });
                module_summary = format!(
                    "{}: {}{}",
                    composite_double_like_route
                        .as_ref()
                        .map(|(_, module)| module.as_str())
                        .unwrap_or("CPU route"),
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" + {}", aux_cpu.join(" + "))
                    }
                );
            }
            "GpuOnly" => {
                route_summary = if is_composite_double_like_strategy(strategy_kind) {
                    format!(
                        "GpuOnly requested | expected CpuOnly fallback for fixed-candidate validation ({strategy_kind})"
                    )
                } else {
                    "GpuOnly requested | expected CpuOnly fallback for fixed-candidate validation"
                        .to_string()
                };
                module_summary = format!(
                    "{}: {}{}",
                    if is_composite_double_like_strategy(strategy_kind) {
                        format!("Expected CPU fallback route ({strategy_kind})")
                    } else {
                        "Expected CPU fallback route".to_string()
                    },
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else {
                        format!(" + {}", aux_cpu.join(" + "))
                    }
                );
                fallback_summary = "GpuOnly is not supported for fixed-candidate Double-MA validation; use GpuKernel, GpuHybridKernel, or Auto.".to_string();
            }
            "GpuHybridKernel" => {
                route_summary = composite_double_like_route
                    .as_ref()
                    .map(|(route, _)| route.clone())
                    .unwrap_or_else(|| {
                        "GpuHybridKernel requested | expected GPU main route (GpuKernel-compatible)"
                            .to_string()
                    });
                module_summary = format!(
                    "{}: {}{}",
                    composite_double_like_route
                        .as_ref()
                        .map(|(_, module)| module.as_str())
                        .unwrap_or("GPU main route"),
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else if composite_double_like_target
                        .is_some_and(|(target, _)| target == "CpuOnly")
                    {
                        format!(" + {}", aux_cpu.join(" + "))
                    } else {
                        format!(" | auxiliary CPU tails: {}", aux_cpu.join(" + "))
                    }
                );
            }
            "GpuKernel" => {
                route_summary = composite_double_like_route
                    .as_ref()
                    .map(|(route, _)| route.clone())
                    .unwrap_or_else(|| "GpuKernel requested | expected GPU main route".to_string());
                module_summary = format!(
                    "{}: {}{}",
                    composite_double_like_route
                        .as_ref()
                        .map(|(_, module)| module.as_str())
                        .unwrap_or("GPU main route"),
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else if composite_double_like_target
                        .is_some_and(|(target, _)| target == "CpuOnly")
                    {
                        format!(" + {}", aux_cpu.join(" + "))
                    } else {
                        format!(" | auxiliary CPU tails: {}", aux_cpu.join(" + "))
                    }
                );
            }
            _ => {
                let auto_composite_double_like_route = composite_double_like_expected_route_text(
                    strategy_kind,
                    validation_req,
                    backend,
                    "auto",
                );
                route_summary = auto_composite_double_like_route
                    .as_ref()
                    .map(|(route, _)| route.clone())
                    .unwrap_or_else(|| {
                        "Auto requested | expected GPU main route when available".to_string()
                    });
                module_summary = format!(
                    "{}: {}{}",
                    auto_composite_double_like_route
                        .as_ref()
                        .map(|(_, module)| module.as_str())
                        .unwrap_or("Expected route"),
                    enabled_main.join(" + "),
                    if aux_cpu.is_empty() {
                        String::new()
                    } else if composite_double_like_target
                        .is_some_and(|(target, _)| target == "CpuOnly")
                    {
                        format!(" + {}", aux_cpu.join(" + "))
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
            && !composite_double_like_target.is_some_and(|(target, _)| target == "CpuOnly")
        {
            route_note_summary = format!(
                "Main GPU route is preserved; {} currently run as intentional auxiliary CPU work.",
                aux_cpu.join(" + ")
            );
        }
    } else {
        let mut warning_notes = Vec::new();
        let composite_profile = composite_profile_label_for_expected_summary(strategy_kind);
        let composite_route_detail = describe_composite_route_detail(strategy_kind, backend);
        let composite_mo_cpu_preferred = strategy_kind == "CompositeMo";
        if backend == "CpuOnly" {
            route_summary =
                format!("CpuOnly requested | CPU composite route expected ({composite_profile})");
            module_summary = format!(
                "CPU route ({composite_profile}): {}{}",
                enabled_main.join(" + "),
                if aux_cpu.is_empty() {
                    gpu_native_optional_module_suffix.clone()
                } else {
                    format!(" + {}", aux_cpu.join(" + "))
                }
            );
        } else if let Some(auto_pref) =
            describe_composite_auto_cpu_preference(strategy_kind, backend)
        {
            route_summary = format!(
                "Auto requested | expected CPU-preferred composite route ({composite_profile})"
            );
            module_summary = format!(
                "Expected CPU route ({composite_profile}): {}{}",
                enabled_main.join(" + "),
                if aux_cpu.is_empty() {
                    gpu_native_optional_module_suffix.clone()
                } else {
                    format!(" + {}", aux_cpu.join(" + "))
                }
            );
            fallback_summary = auto_pref;
        } else if composite_mo_cpu_preferred {
            route_summary = format!(
                "{backend} requested | expected CPU-preferred composite route ({composite_profile})"
            );
            module_summary = format!(
                "Expected CPU route ({composite_profile}): {}{}",
                enabled_main.join(" + "),
                if aux_cpu.is_empty() {
                    gpu_native_optional_module_suffix.clone()
                } else {
                    format!(" + {}", aux_cpu.join(" + "))
                }
            );
            if let Some(route_detail) = composite_route_detail {
                route_note_summary = route_detail;
            }
        } else {
            route_summary =
                format!("{backend} requested | expected GPU composite route ({composite_profile})");
            module_summary = format!(
                "Expected GPU composite route ({composite_profile}): {}{}",
                enabled_main.join(" + "),
                if aux_cpu.is_empty() {
                    gpu_native_optional_module_suffix.clone()
                } else {
                    format!(" + {}", aux_cpu.join(" + "))
                }
            );
            if let Some(route_detail) = composite_route_detail {
                route_note_summary = route_detail;
            }
        }
        if !aux_cpu.is_empty() && backend != "CpuOnly" {
            warning_notes.push(format!(
                "{} may not share the same execution route as the main validation pass; confirm post-run route notes because these are intentional auxiliary CPU work today.",
                aux_cpu.join(" + ")
            ));
        }
        warning_summary = warning_notes.join(" | ");
        if warning_summary.is_empty() {
            warning_summary = "none".to_string();
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
        if mode == "full_stack" {
            cross_summary = format!("{base} | full stack finalists (cap=4)");
            if !is_double_like {
                let cross_warning =
                    "Composite cross reruns use full-stack validation modules for the bounded finalist set.";
                warning_summary = if warning_summary == "none" {
                    cross_warning.to_string()
                } else {
                    format!("{warning_summary} | {cross_warning}")
                };
            }
        } else if is_double_like && mode == "preserved_finalists" {
            cross_summary = format!("{base} | finalists only (cap=4)");
            let cross_warning = "Preserved-finalists cross reruns keep CSCV/PBO + hard gates; auxiliary cross reruns omit walk-forward, SPA, reopt WF, perturb, and trade bootstrap.";
            warning_summary = if warning_summary == "none" {
                cross_warning.to_string()
            } else {
                format!("{warning_summary} | {cross_warning}")
            };
        } else if is_double_like && mode == "lightweight" {
            cross_summary = format!("{base} | lightweight reruns");
        } else if !is_double_like {
            cross_summary = if mode == "preserved_finalists" {
                format!("{base} | finalists only (cap=4)")
            } else {
                format!("{base} | lightweight reruns")
            };
            let cross_warning = if mode == "preserved_finalists" {
                "Composite fixed-candidate main-route validation does not support inline cross_ticker; preserved-finalists composite cross reruns keep CSCV/PBO + hard gates and omit walk_forward, reopt_walk_forward, spa, perturb, and trade_bootstrap.".to_string()
            } else {
                "Composite fixed-candidate main-route validation does not support inline cross_ticker; composite cross reruns are lightweight and omit walk_forward, reopt_walk_forward, spa, perturb, trade_bootstrap, cscv_pbo, and hard gates.".to_string()
            };
            warning_summary = if warning_summary == "none" {
                cross_warning
            } else {
                format!("{warning_summary} | {cross_warning}")
            };
        } else {
            cross_summary = base;
        }
    }

    ExpectedExecutionSummary {
        route_summary,
        cross_summary,
        module_summary,
        category_summary,
        fallback_summary,
        route_note_summary,
        disabled_module_summary,
        cross_omitted_summary,
        warning_summary: if warning_summary.is_empty() {
            "none".to_string()
        } else {
            warning_summary
        },
    }
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
        let approval_label = row.approval_label.clone();
        let params_summary = row.params_summary.clone();
        for bucket in &mut row.regime_bucket_rows {
            bucket.rank = row.rank;
            bucket.approval_label = approval_label.clone();
            bucket.params_summary = params_summary.clone();
        }
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
    let has_run_verdict = parse_run_verdict(validation_res).is_some();
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
            let bootstrap_prob_pnl_positive = bootstrap_iid_metric(
                cand,
                "/trade_bootstrap/prob_pnl_positive",
                "prob_pnl_positive",
            );
            let bootstrap_pnl_p05 =
                bootstrap_iid_metric(cand, "/trade_bootstrap/pnl_p05", "pnl_p05");
            let bootstrap_pnl_p50 =
                bootstrap_iid_metric(cand, "/trade_bootstrap/pnl_p50", "pnl_p50");
            let bootstrap_dd_p95 =
                bootstrap_iid_metric(cand, "/trade_bootstrap/max_dd_p95", "max_dd_p95");
            let bootstrap_stationary_mean_block_len = bootstrap_stationary_block_len(cand);
            let bootstrap_stationary_pnl_p05 =
                bootstrap_stationary_metric(cand, "/trade_bootstrap/stationary_pnl_p05", "pnl_p05");
            let bootstrap_stationary_dd_p95 = bootstrap_stationary_metric(
                cand,
                "/trade_bootstrap/stationary_max_dd_p95",
                "max_dd_p95",
            );
            let monte_carlo_mode = monte_carlo_mode_label(
                bootstrap_prob_pnl_positive,
                bootstrap_pnl_p05,
                bootstrap_dd_p95,
                bootstrap_stationary_mean_block_len,
                bootstrap_stationary_pnl_p05,
                bootstrap_stationary_dd_p95,
            );
            let monte_carlo_sample_count = monte_carlo_sample_count(cand);
            let regime_bucket_rows = parse_regime_bucket_rows(cand);
            let regime_stable_bucket_count =
                opt_u64(cand, "/regime_slice_robustness/stable_bucket_count").map(|v| v as usize);
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
                bootstrap_prob_pnl_positive,
                bootstrap_pnl_p05,
                bootstrap_pnl_p50,
                bootstrap_dd_p95,
                bootstrap_stationary_mean_block_len,
                bootstrap_stationary_pnl_p05,
                bootstrap_stationary_dd_p95,
                monte_carlo_mode,
                monte_carlo_prob_pnl_positive: bootstrap_prob_pnl_positive,
                monte_carlo_pnl_p05: bootstrap_pnl_p05,
                monte_carlo_pnl_p50: bootstrap_pnl_p50,
                monte_carlo_max_dd_p95: bootstrap_dd_p95,
                monte_carlo_stationary_mean_block_len: bootstrap_stationary_mean_block_len,
                monte_carlo_stationary_pnl_p05: bootstrap_stationary_pnl_p05,
                monte_carlo_stationary_max_dd_p95: bootstrap_stationary_dd_p95,
                monte_carlo_sample_count,
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

fn summarize_actual_skipped_modules(model: &ValidationExportModel) -> String {
    let mut skipped = Vec::new();
    if let Some(cscv) = model.cscv_pbo.as_ref() {
        if !cscv.skipped_reason.is_empty() {
            skipped.push(format!("cscv_pbo: {}", cscv.skipped_reason));
        }
    }
    if let Some(spa) = model.spa.as_ref() {
        if !spa.skipped_reason.is_empty() {
            skipped.push(format!("spa: {}", spa.skipped_reason));
        }
    }
    summary_or_dash(skipped, " | ")
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
    let (actual_fallback_summary, actual_route_note_summary) =
        split_actual_route_reason(&model.backend_route, &model.backend_fallback_reason);
    model.actual_fallback_summary = actual_fallback_summary;
    model.actual_route_note_summary = actual_route_note_summary;
    if let Some(validation_req) = report.pointer("/validation/req") {
        model.min_psr = opt_f64(validation_req, "/min_psr");
        model.min_dsr = opt_f64(validation_req, "/min_dsr");
        model.min_cscv_oos_rank_frac = opt_f64(validation_req, "/min_cscv_oos_rank_frac");
        model.min_cscv_oos_metric = opt_f64(validation_req, "/min_cscv_oos_metric");
        model.max_pbo = opt_f64(validation_req, "/max_pbo");
        model.actual_disabled_module_summary =
            summary_or_dash(describe_disabled_modules(validation_req), " + ");
        model.actual_cross_omitted_summary = summary_or_dash(
            describe_cross_omitted_modules(&model.strategy_kind, validation_req),
            " + ",
        );
        let aux_cpu = if !is_double_like_strategy(&model.strategy_kind)
            && is_composite_gpu_native_advanced_strategy(&model.strategy_kind)
        {
            Vec::new()
        } else {
            describe_aux_cpu_tails(validation_req)
        };
        if model.backend_used.eq_ignore_ascii_case("gpu")
            && !model.backend_route.contains("->CpuOnly")
            && !aux_cpu.is_empty()
        {
            model.actual_aux_tail_summary = aux_cpu.join(" + ");
        }
        let report_status = normalize_text(
            report
                .pointer("/validation/acceleration_status")
                .and_then(Value::as_str),
        );
        model.validation_acceleration_status = if report_status.is_empty() {
            build_validation_acceleration_status(
                &model.backend_used,
                &model.backend_route,
                &model.requested_backend,
                &model.backend_fallback_reason,
                model.actual_aux_tail_summary != "-",
            )
        } else {
            report_status
        };
        model.actual_skipped_module_summary = summarize_actual_skipped_modules(&model);
        let expected = build_expected_execution_summary(&model.strategy_kind, Some(validation_req));
        model.expected_route_summary = expected.route_summary;
        model.expected_cross_summary = expected.cross_summary;
        model.expected_module_summary = expected.module_summary;
        model.expected_category_summary = expected.category_summary;
        model.expected_fallback_summary = expected.fallback_summary;
        model.expected_route_note_summary = expected.route_note_summary;
        model.expected_disabled_module_summary = expected.disabled_module_summary;
        model.expected_cross_omitted_summary = expected.cross_omitted_summary;
        model.expected_warning_summary = expected.warning_summary;
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
    let requested_backend = normalize_text(
        validation_res
            .get("requested_backend")
            .and_then(Value::as_str),
    );
    let backend_used = normalize_text(validation_res.get("backend_used").and_then(Value::as_str));
    let backend_route = normalize_text(validation_res.get("backend_route").and_then(Value::as_str));
    let backend_fallback_reason = normalize_text(
        validation_res
            .get("backend_fallback_reason")
            .and_then(Value::as_str),
    );
    let validation_acceleration_status = build_validation_acceleration_status(
        &backend_used,
        &backend_route,
        &requested_backend,
        &backend_fallback_reason,
        false,
    );
    let mut model = ValidationExportModel {
        ts,
        data_id,
        strategy_kind,
        runtime_ms: validation_res.get("runtime_ms").and_then(Value::as_u64),
        requested_backend,
        validation_acceleration_status,
        validation_residency_tier: normalize_text(
            validation_res
                .get("validation_residency_tier")
                .and_then(Value::as_str),
        ),
        backend_used,
        backend_route,
        backend_fallback_reason,
        actual_fallback_summary: "-".to_string(),
        actual_route_note_summary: "-".to_string(),
        actual_aux_tail_summary: "-".to_string(),
        actual_disabled_module_summary: "-".to_string(),
        actual_skipped_module_summary: "-".to_string(),
        actual_cross_omitted_summary: "-".to_string(),
        expected_route_summary: "validation request unavailable".to_string(),
        expected_cross_summary: "not available".to_string(),
        expected_module_summary: "-".to_string(),
        expected_category_summary: "-".to_string(),
        expected_fallback_summary: "-".to_string(),
        expected_route_note_summary: "-".to_string(),
        expected_disabled_module_summary: "-".to_string(),
        expected_cross_omitted_summary: "-".to_string(),
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
        validation_provenance: parse_validation_provenance(validation_res),
        validation_evidence_pack: parse_validation_evidence_pack(validation_res),
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_candidate(
        robust_score: f64,
        passed: bool,
        track_record_bars: u64,
        min_track_record_length_bars: u64,
        params: Value,
        trend_pnl: f64,
        trend_sharpe: f64,
    ) -> Value {
        json!({
            "params": params,
            "validation_verdict": {
                "passed": passed
            },
            "track_record_bars": track_record_bars,
            "min_track_record_length_bars": min_track_record_length_bars,
            "robust_score": robust_score,
            "in_sample": {
                "pnl": robust_score * 10.0,
                "sharpe": robust_score,
                "max_dd": -0.25,
                "trades": 12.0
            },
            "regime_slice_robustness": {
                "trend": {
                    "pnl": trend_pnl,
                    "sharpe": trend_sharpe,
                    "bars": 50
                },
                "range": {
                    "pnl": 0.1,
                    "sharpe": 0.05,
                    "bars": 40
                }
            }
        })
    }

    #[test]
    fn composite_lightweight_cross_summary_matches_frontend_policy() {
        let req = json!({
            "backend": { "backend": "Auto" },
            "cross_ticker": {
                "mode": "lightweight",
                "datasets": [
                    { "csv_path": "C:/data/SPY.csv" },
                    { "csv_path": "C:/data/QQQ.csv" }
                ]
            }
        });

        let summary = build_expected_execution_summary("Cdmc", Some(&req));
        assert!(summary
            .cross_summary
            .contains("2 datasets | lightweight | lightweight reruns"));
        assert!(summary.warning_summary.contains(
            "Composite fixed-candidate main-route validation does not support inline cross_ticker; composite cross reruns are lightweight"
        ));
        assert!(summary.cross_omitted_summary.contains("walk_forward"));
        assert!(summary.cross_omitted_summary.contains("cscv_pbo"));
        assert!(summary.cross_omitted_summary.contains("hard gates"));
    }

    #[test]
    fn composite_preserved_finalists_cross_summary_reports_cap_limited_finalists_only() {
        let req = json!({
            "backend": { "backend": "Auto" },
            "cross_ticker": {
                "mode": "preserved_finalists",
                "datasets": [
                    { "csv_path": "C:/data/SPY.csv" }
                ]
            }
        });

        let summary = build_expected_execution_summary("CompositeCpu", Some(&req));
        assert!(summary
            .cross_summary
            .contains("1 dataset | preserved_finalists | finalists only (cap=4)"));
        assert!(summary
            .warning_summary
            .contains("preserved-finalists composite cross reruns keep CSCV/PBO + hard gates"));
        assert!(summary.cross_omitted_summary.contains("walk_forward"));
        assert!(!summary.cross_omitted_summary.contains("cscv_pbo"));
        assert!(!summary.cross_omitted_summary.contains("hard gates"));
    }

    #[test]
    fn composite_full_stack_cross_summary_preserves_all_modules() {
        let req = json!({
            "backend": { "backend": "Auto" },
            "cross_ticker": {
                "mode": "full_stack",
                "datasets": [
                    { "csv_path": "C:/data/SPY.csv" }
                ]
            }
        });

        let summary = build_expected_execution_summary("CompositeCpu", Some(&req));
        assert!(summary
            .cross_summary
            .contains("1 dataset | full_stack | full stack finalists (cap=4)"));
        assert!(summary
            .warning_summary
            .contains("Composite cross reruns use full-stack validation modules"));
        assert_eq!(summary.cross_omitted_summary, "-");
    }

    #[test]
    fn composite_auto_cpu_expected_labels_use_profile_label() {
        let req = json!({
            "backend": { "backend": "Auto" }
        });

        let summary = build_expected_execution_summary("Carm", Some(&req));
        assert!(summary
            .route_summary
            .contains("expected CPU-preferred composite route (CARM)"));
        assert!(summary
            .module_summary
            .contains("Expected CPU route (CARM):"));
    }

    #[test]
    fn composite_mo_validation_remains_cpu_preferred() {
        let req = json!({
            "backend": { "backend": "GpuKernel" }
        });

        let summary = build_expected_execution_summary("CompositeMo", Some(&req));
        assert!(summary
            .route_summary
            .contains("expected CPU-preferred composite route (CompositeMo)"));
        assert!(summary.route_note_summary.contains(
            "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract"
        ));
        assert!(summary
            .category_summary
            .contains("CPU-only: base reruns + hard gates"));
    }

    #[test]
    fn plain_composite_cpu_gpu_route_summary_remains_explicit() {
        let req = json!({
            "backend": { "backend": "GpuKernel" }
        });

        let summary = build_expected_execution_summary("CompositeCpu", Some(&req));
        assert!(summary
            .route_summary
            .contains("expected GPU composite route (base composite)"));
        assert!(summary
            .module_summary
            .contains("Expected GPU composite route (base composite):"));
        assert!(summary.route_note_summary.contains(
            "Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU."
        ));
    }

    #[test]
    fn composite_trend_auto_expected_summary_tracks_cpu_and_hybrid_validation_routes() {
        let kernel_req = json!({
            "backend": { "backend": "Auto" },
            "candidates": [
                {
                    "strategy": {
                        "trend_quality_use_adx": false
                    }
                }
            ]
        });
        let hybrid_req = json!({
            "backend": { "backend": "Auto" },
            "candidates": [
                {
                    "strategy": {
                        "trend_quality_use_adx": true
                    }
                }
            ]
        });

        let kernel_summary = build_expected_execution_summary("CompositeTrend", Some(&kernel_req));
        assert!(kernel_summary
            .route_summary
            .contains("expected CPU route (CompositeTrend)"));
        assert!(kernel_summary
            .module_summary
            .contains("CPU route (CompositeTrend):"));
        assert!(kernel_summary.category_summary.contains("CPU-only:"));

        let hybrid_summary = build_expected_execution_summary("CompositeTrend", Some(&hybrid_req));
        assert!(hybrid_summary
            .route_summary
            .contains("expected GPU hybrid main route (CompositeTrend) when available"));
        assert!(hybrid_summary
            .module_summary
            .contains("Expected GPU hybrid route (CompositeTrend) when available:"));
    }

    #[test]
    fn breakout_momentum_expected_summary_reroutes_explicit_kernel_request_when_adx_is_on() {
        let req = json!({
            "backend": { "backend": "GpuKernel" },
            "candidates": [
                {
                    "strategy": {
                        "breakout_use_adx": true
                    }
                }
            ]
        });

        let summary = build_expected_execution_summary("BreakoutMomentum", Some(&req));
        assert!(summary
            .route_summary
            .contains("GpuKernel requested | expected GPU hybrid main route (BreakoutMomentum)"));
        assert!(summary
            .module_summary
            .contains("Expected GPU hybrid route (BreakoutMomentum):"));
    }

    #[test]
    fn composite_double_like_cpu_and_gpuonly_expected_summaries_keep_strategy_labels() {
        let cpu_req = json!({
            "backend": { "backend": "CpuOnly" }
        });
        let gpu_only_req = json!({
            "backend": { "backend": "GpuOnly" }
        });

        let cpu_summary = build_expected_execution_summary("CompositeTrend", Some(&cpu_req));
        assert!(cpu_summary
            .route_summary
            .contains("CpuOnly requested | expected CPU route (CompositeTrend)"));
        assert!(cpu_summary
            .module_summary
            .contains("CPU route (CompositeTrend):"));

        let gpu_only_summary =
            build_expected_execution_summary("BreakoutMomentum", Some(&gpu_only_req));
        assert!(gpu_only_summary.route_summary.contains(
            "GpuOnly requested | expected CpuOnly fallback for fixed-candidate validation (BreakoutMomentum)"
        ));
        assert!(gpu_only_summary
            .module_summary
            .contains("Expected CPU fallback route (BreakoutMomentum):"));
    }

    #[test]
    fn parse_run_verdict_requires_explicit_passed_and_approved_flags() {
        let placeholder = json!({
            "run_verdict": {
                "approved_candidate_count": 1
            }
        });
        assert!(parse_run_verdict(&placeholder).is_none());

        let partial = json!({
            "run_verdict": {
                "passed": true,
                "approved_candidate_count": 2
            }
        });
        assert!(parse_run_verdict(&partial).is_none());

        let complete = json!({
            "run_verdict": {
                "passed": true,
                "approved": false,
                "approved_candidate_count": 2
            }
        });
        let verdict = parse_run_verdict(&complete).expect("complete run verdict should parse");
        assert!(verdict.passed);
        assert!(!verdict.approved);
        assert_eq!(verdict.approved_candidate_count, 2);
    }

    #[test]
    fn rows_derive_legacy_bootstrap_metrics_from_monte_carlo_when_absent() {
        let validation_res = json!({
          "data_id": "mc",
          "run_verdict": { "passed": true, "approved": true },
          "candidates": [{
            "params": { "fast_len": 10, "slow_len": 40 },
            "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
            "robust_score": 1.23,
            "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
            "monte_carlo": {
              "schema_version": 1,
              "simulations": 500,
              "trade_count": 12,
              "tail_alpha": 0.05,
              "modes": {
                "trade_iid": {
                  "simulations": 500,
                  "prob_pnl_positive": 0.77,
                  "pnl_p05": -0.11,
                  "pnl_tail_mean_p05": -0.13,
                  "pnl_p50": 0.22,
                  "pnl_p95": 0.44,
                  "max_dd_p50": 0.2,
                  "max_dd_p95": 0.33,
                  "max_dd_tail_mean_p95": 0.36
                },
                "trade_stationary": {
                  "simulations": 500,
                  "prob_pnl_positive": 0.7,
                  "pnl_p05": -0.2,
                  "pnl_tail_mean_p05": -0.24,
                  "pnl_p50": 0.18,
                  "pnl_p95": 0.39,
                  "max_dd_p50": 0.25,
                  "max_dd_p95": 0.41,
                  "max_dd_tail_mean_p95": 0.45,
                  "stationary_mean_block_len": 8
                }
              },
              "advisory": { "passed": true }
            }
          }]
        });

        let model = build_model_from_validation_res(&validation_res, Some(10)).expect("model");
        let row = model.all_rows.first().expect("row");
        assert_eq!(row.bootstrap_prob_pnl_positive, Some(0.77));
        assert_eq!(row.bootstrap_pnl_p05, Some(-0.11));
        assert_eq!(row.bootstrap_pnl_p50, Some(0.22));
        assert_eq!(row.bootstrap_dd_p95, Some(0.33));
        assert_eq!(row.bootstrap_stationary_mean_block_len, Some(8));
        assert_eq!(row.bootstrap_stationary_pnl_p05, Some(-0.2));
        assert_eq!(row.bootstrap_stationary_dd_p95, Some(0.41));
        assert_eq!(row.monte_carlo_mode, "compare_iid_stationary");
        assert_eq!(row.monte_carlo_prob_pnl_positive, Some(0.77));
        assert_eq!(row.monte_carlo_pnl_p05, Some(-0.11));
        assert_eq!(row.monte_carlo_pnl_p50, Some(0.22));
        assert_eq!(row.monte_carlo_max_dd_p95, Some(0.33));
        assert_eq!(row.monte_carlo_stationary_mean_block_len, Some(8));
        assert_eq!(row.monte_carlo_stationary_pnl_p05, Some(-0.2));
        assert_eq!(row.monte_carlo_stationary_max_dd_p95, Some(0.41));
        assert_eq!(row.monte_carlo_sample_count, Some(500));
    }

    #[test]
    fn validation_provenance_is_optional_when_payload_is_absent() {
        let validation_res = json!({
            "data_id": "dataset",
            "candidates": [
                test_candidate(
                    1.0,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 10,
                        "slow_len": 20
                    }),
                    0.4,
                    0.2
                )
            ],
            "run_verdict": {
                "passed": true,
                "approved": true
            }
        });

        let model = build_model_from_validation_res(&validation_res, None)
            .expect("report model should build without provenance");
        assert!(model.validation_provenance.is_none());
    }

    #[test]
    fn validation_evidence_pack_round_trips_into_report_model() {
        let validation_res = json!({
            "data_id": "dataset",
            "validation_evidence_pack": {
                "schema_version": "1-0-0",
                "version": 1,
                "workflow": "double_ma_validation",
                "status": "complete",
                "status_reason": "all requested validation evidence modules produced review metadata",
                "generated_by": "ta_app_core.validation_pipeline",
                "route": {
                    "requested_backend": "GpuKernel",
                    "realized_backend": "Gpu",
                    "route_label": "GpuKernel",
                    "residency_tier": "gpu_kernel"
                },
                "candidate_universe": {
                    "source": "validation_request_candidates",
                    "requested_candidates": 4,
                    "evaluated_candidates": 4,
                    "retained_candidates": 2,
                    "survived_hard_gates": 2,
                    "approved_candidates": 1,
                    "optimization_trials": 58300
                },
                "verdict": {
                    "passed": true,
                    "approved": true,
                    "recommendation": "approved_with_warnings",
                    "summary": "approved"
                },
                "metrics": [
                    {
                        "metric": "cscv_pbo",
                        "availability": "available",
                        "value": 0.14,
                        "threshold": 0.25
                    }
                ],
                "modules": [
                    {
                        "module": "cscv_pbo",
                        "availability": "available",
                        "requested": true,
                        "summary": "evaluated"
                    },
                    {
                        "module": "spa",
                        "availability": "not_requested",
                        "requested": false,
                        "summary": "not_requested"
                    }
                ]
            },
            "candidates": [
                test_candidate(
                    1.0,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 10,
                        "slow_len": 20
                    }),
                    0.4,
                    0.2
                )
            ],
            "run_verdict": {
                "passed": true,
                "approved": true
            }
        });

        let model = build_model_from_validation_res(&validation_res, None)
            .expect("report model should include validation evidence pack");
        let pack = model
            .validation_evidence_pack
            .as_ref()
            .expect("evidence pack");
        assert_eq!(pack.schema_version, "1-0-0");
        assert_eq!(pack.version, Some(1));
        assert_eq!(pack.requested_backend, "GpuKernel");
        assert_eq!(pack.realized_backend, "Gpu");
        assert_eq!(pack.route_label, "GpuKernel");
        assert_eq!(pack.optimization_trials, Some(58_300));
        assert_eq!(pack.verdict_recommendation, "approved_with_warnings");
        assert!(pack
            .module_statuses
            .contains("cscv_pbo=available:evaluated"));
        assert!(pack.metric_summaries.contains("cscv_pbo=0.140000"));
    }

    #[test]
    fn validation_provenance_round_trips_from_validation_res_into_report_model() {
        let validation_res = json!({
            "data_id": "dataset",
            "validation_provenance": {
                "stage_sequence": [
                    "local_path_evidence",
                    "probability_metrics",
                    "cscv_pbo",
                    "cross_ticker_aggregation",
                    "final_verdict_rebuild"
                ],
                "score_channel": "walk_forward_median_guarded",
                "passed_vs_approved_contract": "passed is hard-gate survival; approved requires retained final verdict support",
                "track_record_source": "raw_track_record_bars_with_effective_probability_counts",
                "probability_metric": {
                    "source": "psr_dsr_top_candidate"
                },
                "cscv": {
                    "coverage_state": "full_mask",
                    "candidate_stat_coverage": "all_candidates",
                    "selection_state": "nonzero_selection"
                },
                "cross_mode": "preserved_finalists",
                "cross_summary": {
                    "scope": "success_only",
                    "mode": "preserved_finalists"
                },
                "regime_summary": {
                    "family": "checkpoint_robustness",
                    "role": "descriptive",
                    "semantics": "descriptive-only family-tagged summary",
                    "descriptive_only": true
                }
            },
            "candidates": [
                test_candidate(
                    1.0,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 10,
                        "slow_len": 20
                    }),
                    0.4,
                    0.2
                )
            ],
            "run_verdict": {
                "passed": true,
                "approved": true
            }
        });

        let direct_model = build_model_from_validation_res(&validation_res, None)
            .expect("direct validation result should parse");
        let report = json!({
            "ts": "2026-04-18T00:00:00Z",
            "run": { "req": { "strategy_kind": "DoubleMa" } },
            "validation": {
                "res": validation_res
            }
        });
        let report_model =
            build_model_from_report(&report, None).expect("full report should parse");

        let expected = Some(ExportValidationProvenance {
            stage_sequence:
                "local_path_evidence -> probability_metrics -> cscv_pbo -> cross_ticker_aggregation -> final_verdict_rebuild"
                    .to_string(),
            score_channel: "walk_forward_median_guarded".to_string(),
            passed_vs_approved_contract:
                "passed is hard-gate survival; approved requires retained final verdict support"
                    .to_string(),
            probability_metric_source: "psr_dsr_top_candidate".to_string(),
            track_record_source: "raw_track_record_bars_with_effective_probability_counts".to_string(),
            cscv_coverage_state: "full_mask".to_string(),
            cscv_candidate_stat_coverage: "all_candidates".to_string(),
            cscv_selection_state: "nonzero_selection".to_string(),
            cross_summary_scope: "success_only".to_string(),
            cross_mode: "preserved_finalists".to_string(),
            cross_summary_mode: "preserved_finalists".to_string(),
            regime_summary_family: "checkpoint_robustness".to_string(),
            regime_summary_role: "descriptive".to_string(),
            regime_summary_semantics: "descriptive-only family-tagged summary".to_string(),
            regime_summary_descriptive_only: Some(true),
        });

        assert_eq!(direct_model.validation_provenance, expected);
        assert_eq!(report_model.validation_provenance, expected);
    }

    #[test]
    fn validation_provenance_keeps_passed_true_and_approved_false_distinct() {
        let validation_res = json!({
            "data_id": "dataset",
            "validation_provenance": {
                "score_channel": "selection_robust_score",
                "passed_vs_approved_contract": "passed can remain true when approval is withheld after later retained-state checks",
                "probability_metric_source": "psr_dsr_top_candidate",
                "cscv_coverage_state": "partial_mask",
                "cscv_selection_state": "zero_selected",
                "cross_mode": "lightweight",
                "cross_summary_scope": "success_only",
                "cross_summary_mode": "lightweight",
                "regime_summary_family": "regime_buckets",
                "regime_summary_role": "descriptive",
                "regime_summary_descriptive_only": true,
                "regime_summary_semantics": "descriptive-only; not an approval gate"
            },
            "candidates": [
                test_candidate(
                    1.0,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 10,
                        "slow_len": 20
                    }),
                    0.4,
                    0.2
                )
            ],
            "run_verdict": {
                "passed": true,
                "approved": false,
                "warning_flags": ["spa_not_significant"]
            }
        });

        let model = build_model_from_validation_res(&validation_res, None)
            .expect("report model should build");
        let verdict = model.run_verdict.as_ref().expect("run verdict");
        let provenance = model
            .validation_provenance
            .as_ref()
            .expect("validation provenance");

        assert!(verdict.passed);
        assert!(!verdict.approved);
        assert!(model.approved_rows.is_empty());
        assert_eq!(model.all_rows[0].approval_label, "not_approved");
        assert_eq!(
            model.all_rows[0].approval_reason,
            "run verdict: spa_not_significant"
        );
        assert_eq!(
            provenance.passed_vs_approved_contract,
            "passed can remain true when approval is withheld after later retained-state checks"
        );
        assert_eq!(provenance.score_channel, "selection_robust_score");
        assert_eq!(provenance.cross_mode, "lightweight");
        assert_eq!(provenance.cscv_selection_state, "zero_selected");
        assert_eq!(provenance.cross_summary_scope, "success_only");
        assert_eq!(provenance.regime_summary_family, "regime_buckets");
        assert_eq!(provenance.regime_summary_role, "descriptive");
        assert_eq!(provenance.regime_summary_descriptive_only, Some(true));
    }

    #[test]
    fn build_model_ignores_placeholder_run_verdict_for_run_level_rejection() {
        let validation_res = json!({
            "data_id": "dataset",
            "candidates": [
                test_candidate(
                    1.0,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 10,
                        "slow_len": 20
                    }),
                    0.4,
                    0.2
                )
            ],
            "run_verdict": {
                "approved_candidate_count": 1
            }
        });

        let model = build_model_from_validation_res(&validation_res, None)
            .expect("placeholder run_verdict should remain compatible");
        assert!(model.run_verdict.is_none());
        assert_eq!(model.all_rows.len(), 1);
        assert!(model.all_rows[0].final_approved);
        assert_eq!(model.all_rows[0].approval_label, "approved");
    }

    #[test]
    fn sort_rows_resynchronizes_nested_regime_bucket_metadata() {
        let validation_res = json!({
            "data_id": "dataset",
            "candidates": [
                test_candidate(
                    0.5,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 9,
                        "slow_len": 21
                    }),
                    0.2,
                    0.1
                ),
                test_candidate(
                    1.5,
                    true,
                    140,
                    60,
                    json!({
                        "fast_ma_type": "wma",
                        "slow_ma_type": "ema",
                        "fast_len": 7,
                        "slow_len": 31
                    }),
                    0.6,
                    0.3
                )
            ],
            "run_verdict": {
                "passed": true,
                "approved": true
            }
        });

        let model = build_model_from_validation_res(&validation_res, None)
            .expect("report model should build");
        assert_eq!(model.all_rows.len(), 2);
        let top_row = &model.all_rows[0];
        assert_eq!(top_row.rank, 1);
        assert_eq!(top_row.params_summary, "wma(7) / ema(31)");
        assert_eq!(top_row.approval_label, "approved");
        assert!(!top_row.regime_bucket_rows.is_empty());
        assert!(top_row
            .regime_bucket_rows
            .iter()
            .all(|bucket| bucket.rank == 1));
        assert!(top_row
            .regime_bucket_rows
            .iter()
            .all(|bucket| bucket.approval_label == top_row.approval_label));
        assert!(top_row
            .regime_bucket_rows
            .iter()
            .all(|bucket| bucket.params_summary == top_row.params_summary));
    }

    #[test]
    fn run_level_rejection_resynchronizes_nested_regime_bucket_metadata() {
        let validation_res = json!({
            "data_id": "dataset",
            "candidates": [
                test_candidate(
                    1.0,
                    true,
                    120,
                    60,
                    json!({
                        "fast_ma_type": "ema",
                        "slow_ma_type": "sma",
                        "fast_len": 10,
                        "slow_len": 20
                    }),
                    0.4,
                    0.2
                )
            ],
            "run_verdict": {
                "passed": false,
                "approved": false,
                "fail_reasons": ["cscv_pbo"]
            }
        });

        let model = build_model_from_validation_res(&validation_res, None)
            .expect("report model should build");
        let row = &model.all_rows[0];
        assert!(!row.final_approved);
        assert_eq!(row.approval_label, "not_approved");
        assert_eq!(row.approval_reason, "run verdict: cscv_pbo");
        assert!(!row.regime_bucket_rows.is_empty());
        assert!(row
            .regime_bucket_rows
            .iter()
            .all(|bucket| bucket.rank == row.rank));
        assert!(row
            .regime_bucket_rows
            .iter()
            .all(|bucket| bucket.approval_label == row.approval_label));
        assert!(row
            .regime_bucket_rows
            .iter()
            .all(|bucket| bucket.params_summary == row.params_summary));
    }
}
