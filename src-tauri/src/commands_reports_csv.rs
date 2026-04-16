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
    // Fixed 28-column CSV contract. Keep the current textual encodings stable:
    // `hard_gate_passed` is `true`/`false` or empty, legacy/cross shortlist flags are `1`/`0`,
    // and `params_json` remains one escaped CSV field.
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

    fn parse_csv_records(text: &str) -> Vec<Vec<String>> {
        let mut records = Vec::new();
        let mut record = Vec::new();
        let mut field = String::new();
        let mut chars = text.chars().peekable();
        let mut in_quotes = false;

        while let Some(ch) = chars.next() {
            match ch {
                '"' if in_quotes => {
                    if matches!(chars.peek(), Some('"')) {
                        field.push('"');
                        let _ = chars.next();
                    } else {
                        in_quotes = false;
                    }
                }
                '"' => in_quotes = true,
                ',' if !in_quotes => record.push(std::mem::take(&mut field)),
                '\n' if !in_quotes => {
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                }
                '\r' if !in_quotes => {
                    if matches!(chars.peek(), Some('\n')) {
                        continue;
                    }
                    record.push(std::mem::take(&mut field));
                    records.push(std::mem::take(&mut record));
                }
                _ => field.push(ch),
            }
        }

        if !field.is_empty() || !record.is_empty() {
            record.push(field);
            records.push(record);
        }

        records
    }

    #[test]
    fn csv_escape_quotes_commas_and_newlines() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn shortlist_csv_parses_embedded_newlines_and_preserves_explanatory_fields() {
        let validation_res = json!({
          "data_id": "test",
          "run_verdict": { "passed": true, "approved": true },
          "candidates": [
            {
              "params": { "profile": "alpha", "note": "comma,value" },
              "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
              "robust_score": 1.23,
              "validation_verdict": {
                "passed": true,
                "warning_flags": ["review\nneeds-check"],
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
                "fail_reasons": ["gate\nfailed"]
              }
            }
          ]
        });

        let csv = build_validation_shortlist_csv_text(&validation_res).expect("csv");
        let records = parse_csv_records(&csv);
        assert_eq!(records.len(), 3, "expected header plus both rows");

        let header_fields = &records[0];
        let row_a = &records[1];
        let row_b = &records[2];

        assert_eq!(header_fields.len(), 28);
        assert_eq!(row_a.len(), 28);
        assert_eq!(row_b.len(), 28);
        assert_eq!(row_a[1], "review");
        assert_eq!(row_a[2], "review\nneeds-check");
        assert_eq!(row_a[25], "review\nneeds-check");
        assert_eq!(row_a[26], "");
        assert_eq!(row_b[1], "rejected");
        assert_eq!(row_b[2], "gate\nfailed");
        assert_eq!(row_b[26], "gate\nfailed");
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
        let records = parse_csv_records(&csv);
        let header_fields = &records[0];

        assert_eq!(
            header_fields.as_slice(),
            &[
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

    #[test]
    fn shortlist_csv_dense_row_contract_stays_stable() {
        let validation_res = json!({
          "data_id": "test",
          "run_verdict": { "passed": true, "approved": true },
          "candidates": [{
            "params": {
              "fast_ma_type": "sma",
              "fast_len": 10,
              "slow_ma_type": "ema",
              "slow_len": 40
            },
            "in_sample": { "pnl": 1.5, "sharpe": 2.5, "max_dd": 0.25, "trades": 12 },
            "robust_score": 1.23,
            "robust_pareto": true,
            "psr": 0.91,
            "dsr": 0.82,
            "track_record_bars": 120,
            "min_track_record_length_bars": 100,
            "walk_forward": { "sharpe_median": 1.9, "max_dd_worst": 0.3 },
            "cscv_pbo": {
              "is_selected_frac": 0.75,
              "oos_rank_frac_p50": 0.66,
              "oos_metric_p50": 0.12
            },
            "cross_ticker": { "score": 0.44 },
            "cross_ticker_pareto": true,
            "trade_bootstrap": { "prob_pnl_positive": 0.88 },
            "perturb": { "pnl_mean": 0.22 },
            "validation_verdict": {
              "passed": true,
              "warning_flags": [],
              "fail_reasons": []
            }
          }]
        });

        let csv = build_validation_shortlist_csv_text(&validation_res).expect("csv");
        let records = parse_csv_records(&csv);
        let row = &records[1];

        assert_eq!(
            row,
            &vec![
                "1".to_string(),
                "approved".to_string(),
                "approved".to_string(),
                "true".to_string(),
                "1".to_string(),
                "DOUBLE_MA".to_string(),
                "sma(10) / ema(40)".to_string(),
                "1.23".to_string(),
                "0.91".to_string(),
                "0.82".to_string(),
                "120".to_string(),
                "100".to_string(),
                "1.5".to_string(),
                "2.5".to_string(),
                "0.25".to_string(),
                "12".to_string(),
                "1.9".to_string(),
                "0.3".to_string(),
                "0.75".to_string(),
                "0.66".to_string(),
                "0.12".to_string(),
                "0.44".to_string(),
                "1".to_string(),
                "0.88".to_string(),
                "0.22".to_string(),
                "".to_string(),
                "".to_string(),
                "{\"fast_len\":10,\"fast_ma_type\":\"sma\",\"slow_len\":40,\"slow_ma_type\":\"ema\"}"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn shortlist_csv_errors_when_candidates_are_missing() {
        let validation_res = json!({
          "data_id": "test"
        });

        let err = build_validation_shortlist_csv_text(&validation_res)
            .expect_err("missing candidates should fail");
        assert_eq!(err, "missing validation.res.candidates");
    }

    #[test]
    fn shortlist_csv_errors_when_candidates_are_empty() {
        let validation_res = json!({
          "data_id": "test",
          "candidates": []
        });

        let err = build_validation_shortlist_csv_text(&validation_res)
            .expect_err("empty candidates should fail");
        assert_eq!(err, "validation_res has no candidates");
    }

    #[test]
    fn shortlist_csv_prefers_shortlist_rows_when_model_also_has_all_rows() {
        let candidates: Vec<Value> = (0..4100)
            .map(|idx| {
                json!({
                    "params": {
                        "fast_ma_type": "sma",
                        "fast_len": idx + 1,
                        "slow_ma_type": "ema",
                        "slow_len": idx + 11
                    },
                    "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
                    "robust_score": 5000.0 - idx as f64,
                    "robust_pareto": true,
                    "validation_verdict": {
                        "passed": true,
                        "warning_flags": [],
                        "fail_reasons": []
                    },
                    "track_record_bars": 50,
                    "min_track_record_length_bars": 10
                })
            })
            .collect();

        let validation_res = json!({
            "data_id": "test",
            "run_verdict": { "passed": true, "approved": true },
            "candidates": candidates
        });

        let csv = build_validation_shortlist_csv_text(&validation_res).expect("csv");
        let records = parse_csv_records(&csv);

        assert_eq!(records.len(), 4097, "expected header plus shortlist cap");
        assert_eq!(records[1][0], "1");
        assert_eq!(records[4096][0], "4096");
    }
}
