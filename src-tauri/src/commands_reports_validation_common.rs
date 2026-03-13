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
    pub dataset_labels: String,
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
    pub cross_score: Option<f64>,
    pub cross_pareto: bool,
    pub cscv_selected_frac: Option<f64>,
    pub cscv_oos_rank_frac_p50: Option<f64>,
    pub cscv_oos_metric_p50: Option<f64>,
    pub bootstrap_prob_pnl_positive: Option<f64>,
    pub bootstrap_pnl_p50: Option<f64>,
    pub bootstrap_dd_p95: Option<f64>,
    pub perturb_pnl_worst: Option<f64>,
    pub perturb_pnl_mean: Option<f64>,
    pub perturb_positive_frac: Option<f64>,
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
    let has_cross = candidates
        .iter()
        .any(|cand| cand.get("cross_ticker").and_then(Value::as_object).is_some());

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
                let Some(stats) = candidates[*idx].get("cscv_pbo").and_then(Value::as_object) else {
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
        slice_count: spa
            .get("slice_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        bootstrap_samples: spa
            .get("bootstrap_samples")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        p_value: spa.get("p_value").and_then(Value::as_f64).filter(|v| v.is_finite()),
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
    })
}

fn parse_cscv_summary(validation_res: &Value) -> Option<ExportCscvSummary> {
    let cscv = validation_res.get("cscv_pbo")?.as_object()?;
    Some(ExportCscvSummary {
        slices: cscv.get("slices").and_then(Value::as_u64).unwrap_or(0) as usize,
        slice_bars: cscv
            .get("slice_bars")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        candidates: cscv.get("candidates").and_then(Value::as_u64).unwrap_or(0) as usize,
        combinations: cscv
            .get("combinations")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        pbo: cscv.get("pbo").and_then(Value::as_f64).filter(|v| v.is_finite()),
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
        dataset_labels: labels,
    })
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
            let hard_gate_passed =
                verdict.and_then(|value| value.get("passed")).and_then(Value::as_bool);
            let warning_flags = join_string_array(verdict.and_then(|value| value.get("warning_flags")));
            let fail_reasons = join_string_array(verdict.and_then(|value| value.get("fail_reasons")));
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
                pnl: opt_f64(cand, "/in_sample/pnl").unwrap_or(0.0),
                sharpe: opt_f64(cand, "/in_sample/sharpe").unwrap_or(0.0),
                max_dd: opt_f64(cand, "/in_sample/max_dd").unwrap_or(0.0),
                trades: opt_f64(cand, "/in_sample/trades").unwrap_or(0.0),
                exposure: opt_f64(cand, "/in_sample/exposure"),
                wf_sharpe_median: opt_f64(cand, "/walk_forward/sharpe_median"),
                wf_dd_worst: opt_f64(cand, "/walk_forward/max_dd_worst"),
                cross_score: opt_f64(cand, "/cross_ticker/score"),
                cross_pareto: cand
                    .get("cross_ticker_pareto")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                cscv_selected_frac: opt_f64(cand, "/cscv_pbo/is_selected_frac"),
                cscv_oos_rank_frac_p50: opt_f64(cand, "/cscv_pbo/oos_rank_frac_p50"),
                cscv_oos_metric_p50: opt_f64(cand, "/cscv_pbo/oos_metric_p50"),
                bootstrap_prob_pnl_positive: opt_f64(cand, "/trade_bootstrap/prob_pnl_positive"),
                bootstrap_pnl_p50: opt_f64(cand, "/trade_bootstrap/pnl_p50"),
                bootstrap_dd_p95: opt_f64(cand, "/trade_bootstrap/max_dd_p95"),
                perturb_pnl_worst: opt_f64(cand, "/perturb/pnl_worst"),
                perturb_pnl_mean: opt_f64(cand, "/perturb/pnl_mean"),
                perturb_positive_frac: opt_f64(cand, "/perturb/pnl_positive_frac"),
                warning_flags,
                fail_reasons,
            }
        })
        .collect();

    sort_rows(&mut rows);
    Ok((rows, compatibility_mode))
}

fn run_level_rejection_reason(run_verdict: &ExportRunVerdict) -> String {
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
    extend_group(&mut rows.iter().filter(|row| {
        !row.final_approved && row.hard_gate_passed == Some(true)
    }));
    extend_group(&mut rows.iter().filter(|row| {
        !row.final_approved && row.legacy_shortlisted
    }));
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
    if let Some(validation_req) = report.pointer("/validation/req") {
        model.min_psr = opt_f64(validation_req, "/min_psr");
        model.min_dsr = opt_f64(validation_req, "/min_dsr");
        model.min_cscv_oos_rank_frac = opt_f64(validation_req, "/min_cscv_oos_rank_frac");
        model.min_cscv_oos_metric = opt_f64(validation_req, "/min_cscv_oos_metric");
        model.max_pbo = opt_f64(validation_req, "/max_pbo");
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

    let approved_rows: Vec<ExportCandidateRow> =
        rows.iter().filter(|row| row.final_approved).cloned().collect();
    let shortlist_rows = shortlist_rows(&rows, visible_cap);
    Ok(ValidationExportModel {
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
        all_rows: rows,
        approved_rows,
        shortlist_rows,
    })
}
