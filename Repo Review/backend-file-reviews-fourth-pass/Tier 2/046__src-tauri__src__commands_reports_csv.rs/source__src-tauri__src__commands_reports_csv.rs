use super::commands_reports_validation_common::build_model_from_validation_res;
use serde_json::Value;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse_csv_fields(line: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut current = String::new();
        let mut chars = line.chars().peekable();
        let mut in_quotes = false;

        while let Some(ch) = chars.next() {
            match ch {
                '"' if in_quotes => {
                    if matches!(chars.peek(), Some('"')) {
                        current.push('"');
                        let _ = chars.next();
                    } else {
                        in_quotes = false;
                    }
                }
                '"' => in_quotes = true,
                ',' if !in_quotes => {
                    out.push(current);
                    current = String::new();
                }
                _ => current.push(ch),
            }
        }

        out.push(current);
        out
    }

    #[test]
    fn csv_escape_quotes_commas_and_newlines() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn shortlist_csv_falls_back_to_all_rows_and_preserves_explanatory_fields() {
        let validation_res = json!({
          "data_id": "test",
          "candidates": [
            {
              "params": { "profile": "alpha", "note": "comma,value" },
              "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
              "robust_score": 1.23,
              "validation_verdict": {
                "passed": true,
                "warning_flags": ["review,needs-check"],
                "fail_reasons": []
              }
            },
            {
              "params": { "profile": "beta", "note": "\"quoted\"" },
              "in_sample": { "pnl": 0.5, "sharpe": 1.5, "max_dd": 0.4, "trades": 8 },
              "robust_score": 0.95,
              "validation_verdict": {
                "passed": false,
                "warning_flags": [],
                "fail_reasons": ["gate_failed"]
              }
            }
          ]
        });

        let csv = build_validation_shortlist_csv_text(&validation_res).expect("csv");
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 3, "expected header plus both fallback rows");

        let header_fields = parse_csv_fields(lines[0]);
        let row_a = parse_csv_fields(lines[1]);
        let row_b = parse_csv_fields(lines[2]);

        assert_eq!(header_fields.len(), 28);
        assert_eq!(row_a.len(), 28);
        assert_eq!(row_b.len(), 28);
        assert_eq!(row_a[1], "review");
        assert_eq!(row_a[25], "review,needs-check");
        assert_eq!(row_a[26], "");
        assert_eq!(row_b[1], "rejected");
        assert_eq!(row_b[26], "gate_failed");
        assert_eq!(
            row_b[27],
            "{\"note\":\"\\\"quoted\\\"\",\"profile\":\"beta\"}"
        );
    }

    #[test]
    fn shortlist_csv_header_contract_stays_stable() {
        let validation_res = json!({
          "data_id": "test",
          "candidates": [{
            "params": { "fast_len": 10, "slow_len": 40 },
            "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
            "robust_score": 1.23,
            "validation_verdict": {
              "passed": true,
              "warning_flags": ["warn"],
              "fail_reasons": []
            }
          }]
        });

        let csv = build_validation_shortlist_csv_text(&validation_res).expect("csv");
        let lines: Vec<&str> = csv.lines().collect();
        let header_fields = parse_csv_fields(lines[0]);

        assert_eq!(
            header_fields,
            vec![
                "rank",
                "approval_label",
                "approval_reason",
                "hard_gate_passed",
                "legacy_shortlisted",
                "profile_kind",
                "params_summary",
                "robust_score",
                "psr",
                "dsr",
                "track_record_bars",
                "min_track_record_length_bars",
                "in_sample_pnl",
                "in_sample_sharpe",
                "in_sample_max_dd",
                "in_sample_trades",
                "wf_sharpe_median",
                "wf_dd_worst",
                "cscv_selected_frac",
                "cscv_oos_rank_frac_p50",
                "cscv_oos_metric_p50",
                "cross_score",
                "cross_pareto",
                "bootstrap_prob_pnl_positive",
                "perturb_pnl_mean",
                "warning_flags",
                "fail_reasons",
                "params_json",
            ]
        );
    }
}
