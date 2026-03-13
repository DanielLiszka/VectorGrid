use serde_json::Value;
use super::commands_reports_validation_common::build_model_from_validation_res;

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

pub(crate) fn build_validation_shortlist_csv_blocking(
    validation_res: Value,
) -> Result<String, String> {
    build_validation_shortlist_csv_text(&validation_res)
}

pub(super) fn build_validation_shortlist_csv_text(
    validation_res: &Value,
) -> Result<String, String> {
    let model = build_model_from_validation_res(validation_res, Some(4096))?;
    let rows = if !model.shortlist_rows.is_empty() {
        model.shortlist_rows
    } else {
        model.all_rows
    };

    if rows.is_empty() {
        return Err("validation_res has no candidates".to_string());
    }

    let mut lines: Vec<String> = Vec::with_capacity(rows.len().saturating_add(1));
    lines.push("rank,approval_label,approval_reason,hard_gate_passed,legacy_shortlisted,profile_kind,params_summary,robust_score,psr,dsr,track_record_bars,min_track_record_length_bars,in_sample_pnl,in_sample_sharpe,in_sample_max_dd,in_sample_trades,wf_sharpe_median,wf_dd_worst,cscv_selected_frac,cscv_oos_rank_frac_p50,cscv_oos_metric_p50,cross_score,cross_pareto,bootstrap_prob_pnl_positive,perturb_pnl_mean,warning_flags,fail_reasons,params_json".to_string());

    for row in rows {
        
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            row.rank,
            csv_escape(&row.approval_label),
            csv_escape(&row.approval_reason),
            row.hard_gate_passed
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            if row.legacy_shortlisted { 1 } else { 0 },
            csv_escape(&row.profile_kind),
            csv_escape(&row.params_summary),
            row.robust_score,
            row.psr
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.dsr
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.track_record_bars
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.min_track_record_length_bars
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.pnl,
            row.sharpe,
            row.max_dd,
            row.trades,
            row.wf_sharpe_median
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.wf_dd_worst
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.cscv_selected_frac
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.cscv_oos_rank_frac_p50
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.cscv_oos_metric_p50
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.cross_score
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            if row.cross_pareto { 1 } else { 0 },
            row.bootstrap_prob_pnl_positive
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            row.perturb_pnl_mean
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            csv_escape(&row.warning_flags),
            csv_escape(&row.fail_reasons),
            csv_escape(&row.params_json),
        ));
    }

    Ok(lines.join("\n"))
}
