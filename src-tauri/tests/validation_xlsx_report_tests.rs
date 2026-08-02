use serde_json::json;
use std::io::{Cursor, Read};
use ta_desktop_demo_app::commands::{
    build_report_xlsx_bytes_headless, build_validation_shortlist_csv_headless,
};
use zip::ZipArchive;

fn read_entry(bytes: &[u8], name: &str) -> String {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
    let mut file = zip.by_name(name).expect("zip entry");
    let mut text = String::new();
    file.read_to_string(&mut text).expect("zip text");
    text
}

fn xml_entries(bytes: &[u8]) -> Vec<(String, String)> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
    let mut out = Vec::new();
    for idx in 0..zip.len() {
        let mut file = zip.by_index(idx).expect("zip file");
        let name = file.name().to_string();
        if !name.ends_with(".xml") {
            continue;
        }
        let mut text = String::new();
        let _ = file.read_to_string(&mut text);
        out.push((name, text));
    }
    out
}

fn all_xml(bytes: &[u8]) -> String {
    xml_entries(bytes)
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn chart_titles(bytes: &[u8]) -> Vec<String> {
    let mut titles = Vec::new();
    for (name, xml) in xml_entries(bytes) {
        if !name.starts_with("xl/charts/chart") {
            continue;
        }
        for segment in xml.split("<a:t>").skip(1) {
            if let Some((title, _)) = segment.split_once("</a:t>") {
                titles.push(title.to_string());
            }
        }
    }
    titles
}

fn chart_xml_for_title(bytes: &[u8], title: &str) -> String {
    let title_marker = format!("<a:t>{title}</a:t>");
    xml_entries(bytes)
        .into_iter()
        .find_map(|(name, xml)| {
            (name.starts_with("xl/charts/chart") && xml.contains(&title_marker)).then_some(xml)
        })
        .unwrap_or_else(|| panic!("missing chart titled {title}"))
}

fn row_count(sheet_xml: &str) -> usize {
    sheet_xml.matches("<row ").count()
}

fn decode_xml_text(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn shared_strings(bytes: &[u8]) -> Vec<String> {
    let xml = read_entry(bytes, "xl/sharedStrings.xml");
    xml.split("<si>")
        .skip(1)
        .filter_map(|entry| entry.split_once("</si>").map(|(value, _)| value))
        .map(|entry| {
            let mut value = String::new();
            for text_node in entry.split("<t").skip(1) {
                let Some((_, after_open)) = text_node.split_once('>') else {
                    continue;
                };
                let Some((text, _)) = after_open.split_once("</t>") else {
                    continue;
                };
                value.push_str(&decode_xml_text(text));
            }
            value
        })
        .collect()
}

fn worksheet_cell(bytes: &[u8], sheet_index: usize, cell_ref: &str) -> String {
    let xml = read_entry(bytes, &format!("xl/worksheets/sheet{sheet_index}.xml"));
    let marker = format!("<c r=\"{cell_ref}\"");
    let cell = xml
        .split_once(&marker)
        .and_then(|(_, tail)| tail.split_once("</c>").map(|(value, _)| value))
        .unwrap_or_else(|| panic!("missing worksheet cell {cell_ref}"));
    let raw = cell
        .split_once("<v>")
        .and_then(|(_, tail)| tail.split_once("</v>").map(|(value, _)| value))
        .unwrap_or_else(|| panic!("missing worksheet value {cell_ref}"));
    if cell.contains("t=\"s\"") {
        let index = raw
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid shared string index in {cell_ref}"));
        shared_strings(bytes)
            .get(index)
            .cloned()
            .unwrap_or_else(|| panic!("missing shared string {index} for {cell_ref}"))
    } else {
        raw.to_string()
    }
}

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

fn baseline_report() -> serde_json::Value {
    json!({
      "version": 1,
      "kind": "vectorgrid_report",
      "ts": "2026-03-13T00:00:00Z",
      "run": { "req": { "strategy_kind": "CompositeTrend" } },
      "validation": {
        "req": {
          "backend": { "backend": "GpuKernel", "device_id": 0 },
          "walk_forward": { "window_bars": 12000, "step_bars": 3000 },
          "reopt_walk_forward": { "window_bars": 12000, "step_bars": 3000, "top_n_candidates": 3 },
          "spa": { "slices": 12, "bootstrap_samples": 500, "block_length": 4, "benchmark": "buy_hold" },
          "perturb": { "iterations": 8, "pct_min": 0.015, "pct_max": 0.06, "flip_bool_prob": 0.02 },
          "trade_bootstrap": { "simulations": 500, "stationary_mean_block_len": 9 },
          "cross_ticker": {
            "max_datasets": 2,
            "mode": "preserved_finalists",
            "datasets": [
              { "csv_path": "C:/data/eth.csv", "label": "ETHUSD" },
              { "csv_path": "C:/data/sol.csv", "label": "SOLUSD" }
            ]
          },
          "cscv_pbo": { "slices": 12, "max_combinations": 50000, "threshold": 0.0 },
          "min_psr": 0.50,
          "min_dsr": 0.20,
          "min_cscv_oos_rank_frac": 0.55,
          "min_cscv_oos_metric": 0.0,
          "max_pbo": 0.30
        },
        "res": {
          "data_id": "BTCUSD_1h",
          "runtime_ms": 987,
          "backend_used": "Cpu",
          "backend_route": "Auto->CpuOnly",
          "backend_fallback_reason": "gpu unavailable",
          "run_verdict": {
            "passed": true,
            "approved": true,
            "candidates_tested": 4,
            "candidates_survived_hard_gates": 2,
            "approved_candidate_count": 1,
            "final_recommendation": "single_candidate_supported",
            "warning_flags": [],
            "fail_reasons": [],
            "first_approved_rank": 1,
            "top_candidate_track_record_bars": 300,
            "top_candidate_min_track_record_length_bars": 240,
            "top_candidate_psr": 0.84,
            "top_candidate_dsr": 0.78,
            "pbo": 0.12,
            "max_pbo": 0.30
          },
          "spa": {
            "scope": "validation_search",
            "candidate_count": 4,
            "slice_count": 12,
            "bootstrap_samples": 500,
            "best_candidate_key": "top",
            "best_candidate_mean_diff": 0.10,
            "best_candidate_t_stat": 2.1,
            "p_value": 0.04,
            "passed": true
          },
          "reopt_walk_forward": {
            "windows": 6,
            "top_n_candidates": 3,
            "unique_winners": 2,
            "winner_changes": 1,
            "dominant_winner_frac": 0.67,
            "winner_param_drift_mean": 0.15,
            "winner_param_drift_max": 0.30,
            "train_score_median": 1.10,
            "forward_positive_pnl_frac": 0.67,
            "forward_sharpe_median": 1.14,
            "forward_sharpe_worst": 0.51,
            "forward_max_dd_median": 0.22,
            "forward_max_dd_worst": 0.33,
            "forward_pnl_median": 0.18,
            "forward_pnl_worst": -0.03,
            "forward_trades_median": 10,
            "forward_trades_worst": 5,
            "window_pnl_efficiency_median": 0.82,
            "window_pnl_efficiency_worst": -0.21,
            "window_sharpe_efficiency_median": 0.79,
            "window_sharpe_efficiency_worst": 0.31,
            "window_sharpe_efficiency_std": 0.18
          },
          "cross_ticker": {
            "mode": "preserved_finalists",
            "datasets": [
              { "label": "ETHUSD", "csv_path": "C:/data/eth.csv", "bars": 1000 },
              { "label": "SOLUSD", "csv_path": "C:/data/sol.csv", "bars": 900 }
            ]
          },
          "cscv_pbo": {
            "slices": 12,
            "slice_bars": 240,
            "candidates": 4,
            "combinations": 924,
            "combinations_possible": 924,
            "pbo": 0.12,
            "lambda_p25": -0.2,
            "lambda_p50": 0.1,
            "lambda_p75": 0.3,
            "degradation_slope": -0.1,
            "degradation_intercept": 0.2,
            "degradation_r2": 0.6,
            "threshold": 0.0,
            "prob_oos_below_threshold": 0.24
          },
          "candidates": [
            {
              "params": {
                "fast_ma_type": "ema",
                "fast_len": 12,
                "slow_ma_type": "sma",
                "slow_len": 48,
                "vcc": { "adx_period": 14, "squeeze_period": 20 }
              },
              "in_sample": { "pnl": 0.61, "sharpe": 1.92, "max_dd": 0.16, "trades": 11, "exposure": 0.58 },
              "walk_forward": { "sharpe_median": 1.34, "pnl_median": 0.29, "max_dd_worst": 0.29 },
              "trade_bootstrap": {
                "prob_pnl_positive": 0.81,
                "pnl_p05": -0.04,
                "pnl_p50": 0.22,
                "max_dd_p95": 0.41,
                "stationary_mean_block_len": 9,
                "stationary_pnl_p05": -0.08,
                "stationary_max_dd_p95": 0.46
              },
              "regime_slice_robustness": {
                "vol_window": 48,
                "low_vol": { "bars": 120, "active_frac": 0.42, "pnl": 0.08, "sharpe": 0.61, "max_dd": 0.12, "trades": 3 },
                "mid_vol": { "bars": 118, "active_frac": 0.49, "pnl": 0.11, "sharpe": 0.82, "max_dd": 0.15, "trades": 4 },
                "high_vol": { "bars": 116, "active_frac": 0.57, "pnl": -0.02, "sharpe": -0.18, "max_dd": 0.26, "trades": 4 },
                "trend": { "bars": 175, "active_frac": 0.63, "pnl": 0.14, "sharpe": 0.94, "max_dd": 0.17, "trades": 5 },
                "range": { "bars": 179, "active_frac": 0.31, "pnl": 0.01, "sharpe": 0.09, "max_dd": 0.19, "trades": 2 },
                "worst_bucket_label": "high_vol",
                "worst_sharpe": -0.18,
                "worst_max_dd": 0.26,
                "stable_bucket_count": 4,
                "stable_bucket_frac": 0.8,
                "trend_pass": true,
                "range_pass": true,
                "support_label": "broad"
              },
              "friction_stress": {
                "scenario_count": 4,
                "pass_count": 3,
                "pass_frac": 0.75,
                "worst_scenario_label": "combined_stress",
                "worst_pnl": -0.01,
                "worst_sharpe": -0.04,
                "worst_max_dd": 0.29,
                "mean_rank": 1.5,
                "max_rank_drift": 1
              },
              "cross_ticker": { "score": 0.39 },
              "cross_ticker_pareto": true,
              "cscv_pbo": { "is_selected_frac": 0.72, "oos_rank_frac_p50": 0.69, "oos_metric_p50": 0.21 },
              "perturb": { "pnl_mean": 0.13, "pnl_worst": -0.05, "pnl_positive_frac": 0.67 },
              "psr": 0.84,
              "dsr": 0.78,
              "track_record_bars": 300,
              "min_track_record_length_bars": 240,
              "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
              "robust_score": 1.48,
              "robust_pareto": true
            },
            {
              "params": {
                "fast_ma_type": "ema",
                "fast_len": 10,
                "slow_ma_type": "sma",
                "slow_len": 42,
                "asr": { "angle_deg_min": 10, "z_entry": 2.0 }
              },
              "in_sample": { "pnl": 0.42, "sharpe": 1.31, "max_dd": 0.21, "trades": 13, "exposure": 0.49 },
              "walk_forward": { "sharpe_median": 1.01, "pnl_median": 0.21, "max_dd_worst": 0.31 },
              "trade_bootstrap": {
                "prob_pnl_positive": 0.72,
                "pnl_p05": -0.06,
                "pnl_p50": 0.17,
                "max_dd_p95": 0.43,
                "stationary_mean_block_len": 9,
                "stationary_pnl_p05": -0.10,
                "stationary_max_dd_p95": 0.48
              },
              "regime_slice_robustness": {
                "vol_window": 48,
                "low_vol": { "bars": 120, "active_frac": 0.35, "pnl": 0.02, "sharpe": 0.18, "max_dd": 0.18, "trades": 3 },
                "mid_vol": { "bars": 118, "active_frac": 0.41, "pnl": 0.05, "sharpe": 0.29, "max_dd": 0.22, "trades": 4 },
                "high_vol": { "bars": 116, "active_frac": 0.52, "pnl": -0.05, "sharpe": -0.31, "max_dd": 0.33, "trades": 5 },
                "trend": { "bars": 175, "active_frac": 0.48, "pnl": 0.07, "sharpe": 0.36, "max_dd": 0.23, "trades": 5 },
                "range": { "bars": 179, "active_frac": 0.28, "pnl": -0.02, "sharpe": -0.11, "max_dd": 0.24, "trades": 2 },
                "worst_bucket_label": "high_vol",
                "worst_sharpe": -0.31,
                "worst_max_dd": 0.33,
                "stable_bucket_count": 3,
                "stable_bucket_frac": 0.6,
                "trend_pass": true,
                "range_pass": false,
                "support_label": "trend_only"
              },
              "friction_stress": {
                "scenario_count": 4,
                "pass_count": 2,
                "pass_frac": 0.50,
                "worst_scenario_label": "combined_stress",
                "worst_pnl": -0.04,
                "worst_sharpe": -0.16,
                "worst_max_dd": 0.37,
                "mean_rank": 2.5,
                "max_rank_drift": 2
              },
              "cross_ticker": { "score": 0.28 },
              "cross_ticker_pareto": true,
              "cscv_pbo": { "is_selected_frac": 0.61, "oos_rank_frac_p50": 0.63, "oos_metric_p50": 0.10 },
              "perturb": { "pnl_mean": 0.08, "pnl_worst": -0.11, "pnl_positive_frac": 0.58 },
              "psr": 0.74,
              "dsr": 0.69,
              "track_record_bars": 245,
              "min_track_record_length_bars": 240,
              "validation_verdict": { "passed": true, "warning_flags": ["high_turnover"], "fail_reasons": [] },
              "robust_score": 1.21,
              "robust_pareto": true
            },
            {
              "params": {
                "fast_ma_type": "sma",
                "fast_len": 20,
                "slow_ma_type": "ema",
                "slow_len": 60
              },
              "in_sample": { "pnl": 0.11, "sharpe": 0.53, "max_dd": 0.38, "trades": 8, "exposure": 0.33 },
              "psr": 0.31,
              "dsr": 0.22,
              "track_record_bars": 120,
              "min_track_record_length_bars": 260,
              "validation_verdict": { "passed": false, "warning_flags": ["min_track_record_length_not_met:120/260"], "fail_reasons": ["psr_below_min:0.50"] },
              "robust_score": 0.18,
              "robust_pareto": false
            },
            {
              "params": {
                "fast_ma_type": "sma",
                "fast_len": 8,
                "slow_ma_type": "sma",
                "slow_len": 21,
                "mkcs": { "compression_threshold": 0.01, "reexpansion_threshold": 0.03 }
              },
              "in_sample": { "pnl": 0.08, "sharpe": 0.41, "max_dd": 0.44, "trades": 15, "exposure": 0.40 },
              "psr": 0.29,
              "dsr": 0.18,
              "track_record_bars": 180,
              "min_track_record_length_bars": 220,
              "validation_verdict": { "passed": false, "warning_flags": [], "fail_reasons": ["dsr_below_min:0.20"] },
              "robust_score": 0.07,
              "robust_pareto": false
            }
          ]
        }
      }
    })
}

#[test]
fn validation_csv_and_xlsx_shortlist_share_candidate_identity_and_metrics() {
    let report = baseline_report();
    let validation_res = report
        .pointer("/validation/res")
        .expect("validation result");
    let csv = build_validation_shortlist_csv_headless(validation_res).expect("CSV");
    let records = parse_csv_records(&csv);
    assert_eq!(records[0].len(), 37);
    assert_eq!(records.len(), 5, "header plus four candidate rows");

    let xlsx = build_report_xlsx_bytes_headless(&report, Some(10)).expect("XLSX");
    let csv_row = &records[1];
    let string_cells = [(0usize, "A2"), (5, "D2"), (6, "E2"), (27, "AY2")];
    for (csv_col, xlsx_cell) in string_cells {
        assert_eq!(
            csv_row[csv_col],
            worksheet_cell(&xlsx, 2, xlsx_cell),
            "CSV column {csv_col} vs XLSX {xlsx_cell}"
        );
    }

    let numeric_cells = [
        (7usize, "F2"),
        (8, "G2"),
        (9, "H2"),
        (10, "I2"),
        (11, "J2"),
        (12, "K2"),
        (13, "L2"),
        (14, "M2"),
        (15, "N2"),
        (16, "O2"),
        (17, "P2"),
        (19, "Q2"),
        (20, "R2"),
        (18, "S2"),
        (21, "T2"),
        (23, "V2"),
        (24, "AB2"),
    ];
    for (csv_col, xlsx_cell) in numeric_cells {
        let csv_value = csv_row[csv_col]
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("CSV column {csv_col} should be numeric"));
        let xlsx_value = worksheet_cell(&xlsx, 2, xlsx_cell)
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("XLSX {xlsx_cell} should be numeric"));
        let tolerance = 1e-12_f64.max(csv_value.abs() * 1e-12);
        assert!(
            (csv_value - xlsx_value).abs() <= tolerance,
            "CSV column {csv_col} ({csv_value}) vs XLSX {xlsx_cell} ({xlsx_value})"
        );
    }
}

#[test]
fn workbook_contains_expected_sheets_charts_and_dashboard_content() {
    let bytes = build_report_xlsx_bytes_headless(&baseline_report(), Some(3)).expect("xlsx");
    assert_eq!(&bytes[0..2], b"PK");

    let workbook_xml = read_entry(&bytes, "xl/workbook.xml");
    assert!(workbook_xml.contains("Approved Candidates"));
    assert!(workbook_xml.contains("Shortlist"));
    assert!(workbook_xml.contains("Distributions"));
    assert!(workbook_xml.contains("Audit Data"));
    assert!(workbook_xml.contains("Dashboard"));
    assert!(workbook_xml.contains("Validation Features"));
    assert!(workbook_xml.contains("Robustness Matrix"));
    assert!(workbook_xml.contains("Rejection Diagnostics"));
    assert!(workbook_xml.contains("Report Guide"));
    assert!(workbook_xml.contains("Decision Ledger"));
    assert!(workbook_xml.contains("Downside Stress"));
    assert!(workbook_xml.contains("Regime Audit"));
    assert!(workbook_xml.contains("Candidate Comparison"));
    assert!(workbook_xml.contains("Research Readiness"));
    assert!(workbook_xml.contains("Execution Provenance"));
    assert!(workbook_xml.contains("activeTab=\"4\""));

    let names: Vec<String> = xml_entries(&bytes)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| name.starts_with("xl/charts/chart"))
        .collect();
    assert!(
        names.len() >= 6,
        "expected dashboard and distribution charts, got {:?}",
        names
    );
    let titles = chart_titles(&bytes);
    assert!(titles.iter().any(|title| title == "Approval state counts"));
    assert!(titles
        .iter()
        .any(|title| title == "Top rejection / review reasons"));
    assert!(titles.iter().any(|title| title == "Profile counts"));
    assert!(titles
        .iter()
        .any(|title| title == "Robust score distribution by rank"));
    assert!(titles.iter().any(|title| title == "Sharpe vs MaxDD"));
    assert!(!titles
        .iter()
        .any(|title| title == "WF Sharpe efficiency by robust rank"));
    assert!(titles
        .iter()
        .any(|title| title == "Validation feature status counts"));
    assert!(titles
        .iter()
        .any(|title| title == "Peer robust score by rank"));
    assert!(titles
        .iter()
        .any(|title| title == "Validation ratios and probabilities by rank"));
    assert!(titles
        .iter()
        .any(|title| title == "Probability and rank-fraction gate margins by candidate"));
    assert!(titles
        .iter()
        .any(|title| title == "Rejection and review reason counts"));
    assert!(titles
        .iter()
        .any(|title| title == "Validation outcomes by candidate"));
    assert!(titles
        .iter()
        .any(|title| title == "Worst drawdown by candidate"));
    assert!(!titles
        .iter()
        .any(|title| title == "Drawdown stress components by candidate"));

    let robustness_chart =
        chart_xml_for_title(&bytes, "Validation ratios and probabilities by rank");
    assert!(robustness_chart.contains(">PSR<"));
    assert!(robustness_chart.contains(">DSR<"));
    assert!(!robustness_chart.contains(">CSCV Rank<"));
    assert!(!robustness_chart.contains(">Boot Prob+<"));
    assert!(!robustness_chart.contains(">WFE Sharpe<"));

    let rejection_chart = chart_xml_for_title(
        &bytes,
        "Probability and rank-fraction gate margins by candidate",
    );
    assert!(rejection_chart.contains(">PSR Gap<"));
    assert!(rejection_chart.contains(">DSR Gap<"));
    assert!(rejection_chart.contains(">PBO Margin<"));
    assert!(!rejection_chart.contains(">CSCV Rank Gap<"));

    let approved_sheet = read_entry(&bytes, "xl/worksheets/sheet1.xml");
    let shortlist_sheet = read_entry(&bytes, "xl/worksheets/sheet2.xml");
    let features_sheet = read_entry(&bytes, "xl/worksheets/sheet6.xml");
    let robustness_sheet = read_entry(&bytes, "xl/worksheets/sheet7.xml");
    let diagnostics_sheet = read_entry(&bytes, "xl/worksheets/sheet8.xml");
    let guide_sheet = read_entry(&bytes, "xl/worksheets/sheet9.xml");
    let decision_sheet = read_entry(&bytes, "xl/worksheets/sheet10.xml");
    let downside_sheet = read_entry(&bytes, "xl/worksheets/sheet11.xml");
    assert_eq!(row_count(&approved_sheet), 3, "header + 2 approved rows");
    assert_eq!(row_count(&shortlist_sheet), 4, "header + 3 shortlist rows");
    assert!(approved_sheet.contains("<tableParts"));
    assert!(approved_sheet.contains("<conditionalFormatting"));
    assert!(shortlist_sheet.contains("<tableParts"));
    assert!(shortlist_sheet.contains("<conditionalFormatting"));
    assert!(features_sheet.contains("<tableParts"));
    assert!(features_sheet.contains("<conditionalFormatting"));
    assert!(robustness_sheet.contains("<tableParts"));
    assert!(robustness_sheet.contains("<conditionalFormatting"));
    assert_eq!(worksheet_cell(&bytes, 7, "A19"), "Plateau");
    assert_eq!(worksheet_cell(&bytes, 7, "A20"), "Peer Comparison");
    assert!(diagnostics_sheet.contains("<tableParts"));
    assert!(diagnostics_sheet.contains("<conditionalFormatting"));
    assert!(guide_sheet.contains("<tableParts"));
    assert!(decision_sheet.contains("<tableParts"));
    assert!(decision_sheet.contains("<conditionalFormatting"));
    assert!(downside_sheet.contains("<tableParts"));
    assert!(downside_sheet.contains("<conditionalFormatting"));
    assert!(all_xml(&bytes).contains("tableStyleInfo"));

    let xml = all_xml(&bytes);
    assert!(xml.contains("Validation XLSX Report"));
    assert!(xml.contains("single_candidate_supported"));
    assert!(xml.contains("ETHUSD"));
    assert!(xml.contains("ASR(") || xml.contains("ASR"));
    assert!(xml.contains("VCC(") || xml.contains("VCC"));
    assert!(xml.contains("Visible shortlist capped at 3 rows;"));
    assert!(xml.contains("Charts and audit use the full set of 4 rows."));
    assert!(xml.contains("not run"));
    assert!(xml.contains("Gate Config"));
    assert!(xml.contains("Min DSR"));
    assert!(xml.contains("Ranking Stability"));
    assert!(xml.contains("WF overlap/drift"));
    assert!(xml.contains("CSCV overlap/drift"));
    assert!(xml.contains("Cross overlap/drift"));
    assert!(xml.contains("Stable finalists"));
    assert!(xml.contains("WF PnL Eff"));
    assert!(xml.contains("WF Sharpe Eff"));
    assert!(xml.contains("WF Efficiency"));
    assert!(xml.contains("Median Sharpe Eff"));
    assert!(xml.contains("explanatory + tie-break ordering only; no approval gate"));
    assert!(xml.contains("WFE Sharpe P50"));
    assert!(xml.contains("WFE Sharpe Worst"));
    assert!(xml.contains("Bootstrap P05"));
    assert!(xml.contains("Bootstrap DD95"));
    assert!(xml.contains("Stat Block"));
    assert!(xml.contains("Stat P05"));
    assert!(xml.contains("Stat DD95"));
    assert!(xml.contains("Bootstrap Compare"));
    assert!(xml.contains("iid + stationary closed-trade comparison"));
    assert!(xml.contains("Compare (IID + Stationary)"));
    assert!(xml.contains("Regime Worst"));
    assert!(xml.contains("Regime Sharpe"));
    assert!(xml.contains("Regime MaxDD"));
    assert!(xml.contains("Regime Stable"));
    assert!(xml.contains("Stable Frac"));
    assert!(xml.contains("Regime Flag"));
    assert!(xml.contains("Regime Support"));
    assert!(xml.contains("Regime Robustness"));
    assert!(xml.contains("Worst Bucket"));
    assert!(xml.contains("Support"));
    assert!(
        xml.contains("see Audit Data -&gt; Regime Slice Audit")
            || xml.contains("see Audit Data -> Regime Slice Audit")
    );
    assert!(xml.contains("Regime Slice Audit"));
    assert!(xml.contains("Low Vol"));
    assert!(xml.contains("Broad") || xml.contains("Trend OK") || xml.contains("Mixed"));
    assert!(xml.contains("Friction Stability"));
    assert!(xml.contains("Worst Scenario"));
    assert!(xml.contains("Worst-Scenario Sharpe"));
    assert!(xml.contains("Worst-Scenario MaxDD"));
    assert!(xml.contains("Pass Frac"));
    assert!(xml.contains("Friction Flag"));
    assert!(xml.contains("Shortlist Count"));
    assert!(xml.contains("Semantics"));
    assert!(
        xml.contains("absolute pass: pnl &gt;= 0 and sharpe &gt;= 0")
            || xml.contains("absolute pass: pnl >= 0 and sharpe >= 0")
    );
    assert!(xml.contains("finalist-only fee/spread/slippage stress; absolute pass uses pnl &gt;= 0 and sharpe &gt;= 0")
        || xml.contains("finalist-only fee/spread/slippage stress; absolute pass uses pnl >= 0 and sharpe >= 0"));
    assert!(xml.contains("Approval State"));
    assert!(xml.contains("Top Rejection / Review Reason"));
    assert!(xml.contains("Preserved Finalists"));
    assert!(xml.contains("perturb + trade bootstrap compare (iid + stationary, stat block=9)"));
    assert!(xml.contains("Approval state counts"));
    assert!(xml.contains("Approved"));
    assert!(xml.contains("Rejected"));
    assert!(xml.contains("Rank Robust"));
    assert!(xml.contains("Max Drift"));
    assert!(xml.contains("psr_below_min:0.50"));
    assert!(xml.contains("high_turnover"));
    assert!(xml.contains("Validation Feature Coverage"));
    assert!(xml.contains("Run-by-run inventory of validation modules"));
    assert!(xml.contains("ValidationFeaturesTable"));
    assert!(xml.contains("Final run verdict"));
    assert!(xml.contains("Candidate hard gates"));
    assert!(xml.contains("Closed-trade bootstrap"));
    assert!(xml.contains("Parameter perturbation"));
    assert!(xml.contains("Execution route evidence"));
    assert!(xml.contains("Validation provenance contract"));
    assert!(xml.contains("Top Candidate Robustness Matrix"));
    assert!(xml.contains("Derived from the saved validation artifact"));
    assert!(xml.contains("Peer Comparison"));
    assert!(xml.contains("CSCV PBO"));
    assert!(xml.contains("Bootstrap downside"));
    assert!(xml.contains("Walk-forward efficiency"));
    assert!(xml.contains("Rejection Diagnostics"));
    assert!(xml.contains("Gate distance columns show observed minus required minimum"));
    assert!(xml.contains("PSR Gap"));
    assert!(xml.contains("DSR Gap"));
    assert!(xml.contains("Observed Gate Fails"));
    assert!(xml.contains("Candidate Decision Map"));
    assert!(xml.contains("CandidateDecisionMapTable"));
    assert!(xml.contains("CandidateDecisionLedgerTable"));
    assert!(xml.contains("Decision Ledger Detail"));
    assert!(xml.contains("Downside Stress Summary"));
    assert!(xml.contains("DownsideStressTable"));
    assert!(xml.contains("Worst DD Source"));
    assert!(xml.contains("Worst PnL Source"));
    assert!(xml.contains("CandidateComparisonTable"));
    assert!(xml.contains("ResearchReadinessTable"));
    assert!(xml.contains("DeploymentRealismTable"));
    assert!(xml.contains("RecordedExecutionTable"));
    assert!(xml.contains("ExpectedExecutionPolicyTable"));
    assert!(xml.contains("Research evidence only"));
    assert!(xml.contains("not deployment authorization"));
    assert!(xml.contains("Report Guide"));
    assert!(xml.contains("Quick reference for interpreting the workbook"));
    assert!(xml.contains("Positive is better"));
    assert!(xml.contains("tabColor"));
    assert!(xml.contains("pageSetup"));
}

#[test]
fn workbook_dashboard_links_quote_internal_sheet_targets_with_spaces() {
    let bytes = build_report_xlsx_bytes_headless(&baseline_report(), Some(3)).expect("xlsx");
    let dashboard_sheet = read_entry(&bytes, "xl/worksheets/sheet5.xml");

    assert!(
        dashboard_sheet.contains("location=\"'Approved Candidates'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Approved Candidates&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Audit Data'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Audit Data&apos;!A1\"")
    );
    assert!(dashboard_sheet.contains("location=\"Shortlist!A1\""));
    assert!(dashboard_sheet.contains("location=\"Distributions!A1\""));
    assert!(
        dashboard_sheet.contains("location=\"'Validation Features'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Validation Features&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Robustness Matrix'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Robustness Matrix&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Rejection Diagnostics'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Rejection Diagnostics&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Report Guide'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Report Guide&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Decision Ledger'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Decision Ledger&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Downside Stress'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Downside Stress&apos;!A1\"")
    );
    assert!(
        dashboard_sheet.contains("location=\"'Regime Audit'!A1\"")
            || dashboard_sheet.contains("location=\"&apos;Regime Audit&apos;!A1\"")
    );
    for sheet in [
        "Candidate Comparison",
        "Research Readiness",
        "Execution Provenance",
    ] {
        assert!(
            dashboard_sheet.contains(&format!("location=\"'{sheet}'!A1\""))
                || dashboard_sheet.contains(&format!("location=\"&apos;{sheet}&apos;!A1\""))
        );
    }
}

#[test]
fn optional_chart_series_reappear_only_with_complete_candidate_evidence() {
    let mut report = baseline_report();
    let candidates = report
        .pointer_mut("/validation/res/candidates")
        .and_then(serde_json::Value::as_array_mut)
        .expect("candidate rows");
    for (idx, row) in candidates.iter_mut().enumerate().skip(2) {
        let offset = idx as f64 * 0.01;
        row["walk_forward"] = json!({
            "sharpe_median": 0.42 + offset,
            "pnl_median": 0.06 + offset,
            "max_dd_worst": 0.40 + offset
        });
        row["cscv_pbo"] = json!({
            "is_selected_frac": 0.30 + offset,
            "oos_rank_frac_p50": 0.40 + offset,
            "oos_metric_p50": -0.05 + offset
        });
        row["trade_bootstrap"] = json!({
            "prob_pnl_positive": 0.55 + offset,
            "pnl_p05": -0.12,
            "pnl_p50": 0.04,
            "max_dd_p95": 0.49 + offset,
            "stationary_mean_block_len": 9,
            "stationary_pnl_p05": -0.15,
            "stationary_max_dd_p95": 0.53 + offset
        });
    }

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let titles = chart_titles(&bytes);
    assert!(titles
        .iter()
        .any(|title| title == "WF Sharpe efficiency by robust rank"));
    assert!(titles
        .iter()
        .any(|title| title == "Drawdown stress components by candidate"));

    let robustness_chart =
        chart_xml_for_title(&bytes, "Validation ratios and probabilities by rank");
    assert!(robustness_chart.contains(">CSCV Rank<"));
    assert!(robustness_chart.contains(">Bootstrap Prob+<"));
    assert!(robustness_chart.contains(">WFE Sharpe<"));

    let rejection_chart = chart_xml_for_title(
        &bytes,
        "Probability and rank-fraction gate margins by candidate",
    );
    assert!(rejection_chart.contains(">CSCV Rank Gap<"));

    let downside_chart = chart_xml_for_title(&bytes, "Drawdown stress components by candidate");
    assert!(downside_chart.contains(">MaxDD<"));
    assert!(downside_chart.contains(">WF Worst<"));
    assert!(downside_chart.contains(">Bootstrap DD95<"));
    assert!(downside_chart.contains(">Stationary DD95<"));
    assert!(!downside_chart.contains(">Regime DD<"));
    assert!(!downside_chart.contains(">Friction DD<"));
}

#[test]
fn workbook_dashboard_keeps_slice_audit_and_bootstrap_compare_on_distinct_rows() {
    let bytes = build_report_xlsx_bytes_headless(&baseline_report(), Some(3)).expect("xlsx");
    let xml = all_xml(&bytes);
    let dashboard_sheet = read_entry(&bytes, "xl/worksheets/sheet5.xml");

    assert!(xml.contains("Slice Audit"));
    assert!(xml.contains("Bootstrap Compare"));
    assert!(dashboard_sheet.contains("r=\"74\""));
    assert!(dashboard_sheet.contains("r=\"75\""));
}

#[test]
fn workbook_respects_run_level_rejection_and_zero_approved_rows() {
    let mut report = baseline_report();
    let res = report
        .pointer_mut("/validation/res")
        .and_then(|value| value.as_object_mut())
        .expect("validation res");
    let run_verdict = res
        .get_mut("run_verdict")
        .and_then(|value| value.as_object_mut())
        .expect("run verdict");
    run_verdict.insert("approved".to_string(), json!(false));
    run_verdict.insert("passed".to_string(), json!(true));
    run_verdict.insert(
        "final_recommendation".to_string(),
        json!("candidates_survived_but_search_not_statistically_superior"),
    );
    run_verdict.insert("warning_flags".to_string(), json!(["spa_not_significant"]));
    run_verdict.insert("approved_candidate_count".to_string(), json!(0));
    res.insert(
        "spa".to_string(),
        json!({
          "scope": "validation_search",
          "candidate_count": 4,
          "slice_count": 12,
          "bootstrap_samples": 500,
          "best_candidate_key": "top",
          "best_candidate_mean_diff": 0.10,
          "best_candidate_t_stat": 1.2,
          "p_value": 0.18,
          "passed": false
        }),
    );

    let bytes = build_report_xlsx_bytes_headless(&report, Some(25)).expect("xlsx");
    let approved_sheet = read_entry(&bytes, "xl/worksheets/sheet1.xml");
    let shortlist_sheet = read_entry(&bytes, "xl/worksheets/sheet2.xml");
    assert_eq!(
        row_count(&approved_sheet),
        1,
        "approved sheet should only have header"
    );
    assert_eq!(
        row_count(&shortlist_sheet),
        5,
        "shortlist should retain the visible shortlist context even when final approval is rejected"
    );

    let xml = all_xml(&bytes);
    assert!(xml.contains("spa_not_significant"));
    assert!(xml.contains("run verdict: spa_not_significant"));
}

#[test]
fn workbook_surfaces_iid_only_bootstrap_mode_when_requested() {
    let mut report = baseline_report();
    let trade_bootstrap = report
        .pointer_mut("/validation/req/trade_bootstrap")
        .and_then(|value| value.as_object_mut())
        .expect("trade bootstrap req");
    trade_bootstrap.insert("stationary_mean_block_len".to_string(), json!(0));

    let candidates = report
        .pointer_mut("/validation/res/candidates")
        .and_then(|value| value.as_array_mut())
        .expect("candidates");
    for candidate in candidates {
        if let Some(trade_bootstrap) = candidate
            .get_mut("trade_bootstrap")
            .and_then(|value| value.as_object_mut())
        {
            trade_bootstrap.remove("stationary_mean_block_len");
            trade_bootstrap.remove("stationary_pnl_p05");
            trade_bootstrap.remove("stationary_max_dd_p95");
        }
    }

    let bytes = build_report_xlsx_bytes_headless(&report, Some(25)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("iid-only closed-trade bootstrap"));
    assert!(xml.contains("IID only"));
}

#[test]
fn workbook_supports_legacy_compatibility_mode() {
    let report = json!({
      "version": 1,
      "kind": "vectorgrid_report",
      "ts": "2026-03-13T00:00:00Z",
      "validation": {
        "res": {
          "data_id": "legacy_mode",
          "candidates": [
            {
              "params": { "fast_ma_type": "sma", "fast_len": 9, "slow_ma_type": "ema", "slow_len": 21 },
              "in_sample": { "pnl": 1.0, "sharpe": 1.5, "max_dd": 0.3, "trades": 8 },
              "robust_score": 0.9,
              "robust_pareto": true,
              "cross_ticker": { "score": 0.2 },
              "cross_ticker_pareto": true,
              "cscv_pbo": { "is_selected_frac": 0.7, "oos_rank_frac_p50": 0.6, "oos_metric_p50": 0.1 }
            }
          ]
        }
      }
    });

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(
        xml.contains("Compatibility mode: workbook generated from a legacy validation artifact")
    );
    assert!(xml.contains("legacy shortlist compatibility mode"));
    assert!(xml.contains("not run"));
}

#[test]
fn workbook_supports_all_profile_families_in_strategy_agnostic_mode() {
    let profiles = [
        "asr", "vcc", "vemr", "rocb", "vcet", "mkcs", "kqsr", "carm", "kvdf", "cdmc",
    ];

    for profile in profiles {
        let report = json!({
          "version": 1,
          "kind": "vectorgrid_report",
          "ts": "2026-03-13T00:00:00Z",
          "run": { "req": { "strategy_kind": "CompositeTrend" } },
          "validation": {
            "res": {
              "data_id": format!("{}_dataset", profile),
              "backend_used": "Cpu",
              "run_verdict": {
                "passed": true,
                "approved": true,
                "candidates_tested": 1,
                "candidates_survived_hard_gates": 1,
                "approved_candidate_count": 1,
                "final_recommendation": "single_candidate_supported",
                "warning_flags": [],
                "fail_reasons": [],
                "first_approved_rank": 1
              },
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "ema",
                    "fast_len": 10,
                    "slow_ma_type": "sma",
                    "slow_len": 30,
                    profile: { "alpha": 1.0, "beta": 2.0 }
                  },
                  "in_sample": { "pnl": 0.2, "sharpe": 1.0, "max_dd": 0.2, "trades": 9 },
                  "psr": 0.7,
                  "dsr": 0.6,
                  "track_record_bars": 260,
                  "min_track_record_length_bars": 200,
                  "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
                  "robust_score": 0.9,
                  "robust_pareto": true
                }
              ]
            }
          }
        });

        let bytes = build_report_xlsx_bytes_headless(&report, Some(10))
            .unwrap_or_else(|err| panic!("profile {} should export: {}", profile, err));
        let xml = all_xml(&bytes);
        assert!(
            xml.contains(&profile.to_ascii_uppercase()),
            "profile {} should appear in workbook text",
            profile
        );
    }
}

#[test]
fn workbook_zero_survivor_runs_still_emit_fallback_charts() {
    let report = json!({
      "version": 1,
      "kind": "vectorgrid_report",
      "ts": "2026-03-13T00:00:00Z",
      "run": { "req": { "strategy_kind": "DoubleMa" } },
      "validation": {
        "req": {
          "backend": { "backend": "GpuKernel", "device_id": 0 },
          "perturb": { "iterations": 8, "pct_min": 0.015, "pct_max": 0.06, "flip_bool_prob": 0.02 },
          "trade_bootstrap": { "simulations": 500 },
          "cross_ticker": {
            "max_datasets": 1,
            "mode": "preserved_finalists",
            "datasets": [
              { "csv_path": "C:/data/SPY.csv", "label": "SPY" }
            ]
          }
        },
        "res": {
          "data_id": "zero_survivor",
          "backend_used": "Gpu",
          "backend_route": "GpuKernel",
          "run_verdict": {
            "passed": false,
            "approved": false,
            "candidates_tested": 0,
            "candidates_survived_hard_gates": 0,
            "approved_candidate_count": 0,
            "final_recommendation": "no_candidate_survived",
            "warning_flags": [],
            "fail_reasons": ["no_candidates_survived_hard_gates"]
          },
          "cross_ticker": {
            "mode": "preserved_finalists",
            "datasets": [
              { "label": "SPY", "csv_path": "C:/data/SPY.csv", "bars": 1000 }
            ]
          },
          "candidates": []
        }
      }
    });

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let charts: Vec<String> = xml_entries(&bytes)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| name.starts_with("xl/charts/chart"))
        .collect();
    assert!(
        !charts.is_empty(),
        "expected fallback charts for zero-survivor workbook"
    );

    let titles = chart_titles(&bytes);
    assert!(titles.iter().any(|title| title == "Approval state counts"));
    assert!(titles
        .iter()
        .any(|title| title == "Top rejection / review reasons"));

    let xml = all_xml(&bytes);
    assert!(xml.contains("Run Verdict: no candidate survived"));
    assert!(xml.contains("run verdict: no_candidates_survived_hard_gates"));
    assert!(xml.contains("Preserved Finalists"));
    assert!(xml.contains("Cross-Ticker"));
    assert!(xml.contains("Shortlist Count"));
    assert!(xml.contains("Semantics"));
    assert!(xml.contains("final post-cross shortlist"));
    assert!(xml.contains("GpuKernel"));
    assert!(xml.contains("Execution Notes"));
    assert!(xml.contains("Route Fallback"));
    assert!(xml.contains("Route Note"));
    assert!(xml.contains("Aux/Mixed Tails (inferred)"));
    assert!(xml.contains("Disabled Modules"));
    assert!(xml.contains("Cross Omitted Modules"));
    assert!(xml.contains("Top Warning"));
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Route"));
    assert!(xml.contains("Expected Cross"));
    assert!(xml.contains("Expected Modules"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Fallback"));
    assert!(xml.contains("Expected Route Notes"));
    assert!(xml.contains("Expected Disabled"));
    assert!(xml.contains("Expected Cross Omitted"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains("GpuKernel requested"));
    assert!(xml.contains("1 dataset"));
    assert!(xml.contains("finalists only (cap=4)"));
    assert!(xml.contains(
        "auxiliary CPU tails: perturb + trade bootstrap compare (iid + stationary, stat block=auto)"
    ));
    assert!(xml.contains("GPU-backed reruns/evaluation: base reruns"));
    assert!(xml.contains("CPU statistical/gate reduction: probability metrics + Pareto marking + hard gates + final ordering"));
    assert!(xml.contains("CPU auxiliary tail: perturb + trade_bootstrap"));
    assert!(xml.contains(
        "Main GPU route is preserved; perturb + trade bootstrap compare (iid + stationary, stat block=auto) currently run as intentional auxiliary CPU work."
    ));
    assert!(xml.contains("Preserved-finalists cross reruns keep CSCV/PBO + hard gates"));
    assert!(xml.contains("walk_forward + reopt_walk_forward + spa + perturb + trade_bootstrap"));
}

#[test]
fn workbook_surfaces_route_note_and_skipped_modules_on_dashboard() {
    let report = json!({
      "version": 1,
      "kind": "vectorgrid_report",
      "ts": "2026-03-13T00:00:00Z",
      "run": { "req": { "strategy_kind": "DoubleMa" } },
      "validation": {
        "acceleration_status": "GPU validation completed.",
        "res": {
          "data_id": "gpu_route_note",
          "requested_backend": "GpuHybridKernel",
          "validation_residency_tier": "gpu_hybrid_kernel",
          "backend_used": "Gpu",
          "backend_route": "GpuHybridKernel->GpuKernel",
          "backend_fallback_reason": "GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel.",
          "cscv_pbo": {
            "skipped_reason": "cscv_skipped_not_enough_bars"
          },
          "spa": {
            "skipped_reason": "spa_skipped_not_enough_slices"
          },
          "run_verdict": {
            "passed": false,
            "approved": false,
            "candidates_tested": 1,
            "candidates_survived_hard_gates": 0,
            "approved_candidate_count": 0,
            "final_recommendation": "no_candidate_survived",
            "warning_flags": ["cross_ticker_filtered_all_candidates"],
            "fail_reasons": []
          },
          "candidates": []
        }
      }
    });

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Acceleration Status"));
    assert!(xml.contains("GPU validation completed."));
    assert!(xml.contains("Requested Backend"));
    assert!(xml.contains("GpuHybridKernel"));
    assert!(xml.contains("Residency"));
    assert!(xml.contains("provenance mismatch"));
    assert!(xml.contains("Route Fallback"));
    assert!(xml.contains("Expected Route Notes"));
    assert!(xml.contains("Execution Notes"));
    assert!(xml.contains("cscv_skipped_not_enough_bars"));
    assert!(xml.contains("spa_skipped_not_enough_slices"));
    assert!(xml.contains("Top Warning"));
    assert!(xml.contains("cross_ticker_filtered_all_candidates"));
    assert!(worksheet_cell(&bytes, 15, "B8")
        .contains("terminal route GpuKernel implies gpu_kernel; provenance mismatch"));
    assert_eq!(
        worksheet_cell(&bytes, 15, "B11"),
        "GpuHybridKernel disabled by VECTORBT_ENABLE_GPU_HYBRID_KERNEL=0; using GpuKernel."
    );
    assert_eq!(worksheet_cell(&bytes, 15, "B12"), "-");
}

#[test]
fn workbook_surfaces_disabled_modules_and_disabled_statuses() {
    let mut report = baseline_report();
    let req = report
        .pointer_mut("/validation/req")
        .and_then(|value| value.as_object_mut())
        .expect("validation req");
    req.remove("reopt_walk_forward");
    req.remove("cscv_pbo");
    req.remove("spa");
    req.remove("cross_ticker");

    let res = report
        .pointer_mut("/validation/res")
        .and_then(|value| value.as_object_mut())
        .expect("validation res");
    res.remove("reopt_walk_forward");
    res.remove("cscv_pbo");
    res.remove("spa");
    res.remove("cross_ticker");

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("reopt_walk_forward + cscv_pbo + spa + cross_ticker"));
    assert!(xml.contains("Expected Disabled"));
    assert!(xml.contains("disabled"));
}

#[test]
fn workbook_uses_validation_context_strategy_kind_when_run_req_is_missing() {
    let mut report = baseline_report();
    report.as_object_mut().expect("report").remove("run");
    report["validation"]["req"]["validation_context"] = json!({
      "strategy_kind": "Carm",
      "csv_path": "C:/Rust Projects/ta_desktop_demo/data/AAPL.csv",
      "label": "AAPL"
    });
    report["validation"]["res"]["backend_used"] = json!("Gpu");
    report["validation"]["res"]["backend_route"] = json!("GpuKernel");
    report["validation"]["res"]["backend_fallback_reason"] = serde_json::Value::Null;

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains(
        "CARM GPU validation uses GpuKernel-backed candidate and slice reruns for the fixed-candidate stack."
    ));
    assert!(!xml.contains(">Unknown<"));
}

#[test]
fn workbook_surfaces_composite_route_detail_note_in_expected_execution() {
    let mut report = baseline_report();
    report["run"]["req"]["strategy_kind"] = json!("Carm");
    report["validation"]["res"]["backend_used"] = json!("Gpu");
    report["validation"]["res"]["backend_route"] = json!("GpuKernel");
    report["validation"]["res"]["backend_fallback_reason"] = serde_json::Value::Null;

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains(
        "CARM GPU validation uses GpuKernel-backed candidate and slice reruns for the fixed-candidate stack."
    ));
}

#[test]
fn workbook_surfaces_composite_auto_cpu_preference_note_in_expected_execution() {
    let mut report = baseline_report();
    report["run"]["req"]["strategy_kind"] = json!("Carm");
    report["validation"]["req"]["backend"] = json!({ "backend": "Auto", "device_id": 0 });
    report["validation"]["res"]["backend_used"] = json!("Cpu");
    report["validation"]["res"]["backend_route"] = json!("Auto->CpuOnly");
    report["validation"]["res"]["backend_fallback_reason"] =
        json!("Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on measured workloads.");

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains("Auto requested | expected CPU-preferred composite route (CARM)"));
    assert!(xml.contains("CPU-only: base reruns + hard gates + walk_forward + reopt_walk_forward + cscv_pbo + spa + perturb + trade_bootstrap"));
    assert!(xml
        .contains("Auto composite validation currently prefers CPU for CARM because current advanced-stack benches show CPU faster than GPU on the measured 50k/8 and 100k/32 workloads."));
}

#[test]
fn workbook_surfaces_cdmc_gpu_route_detail_and_composite_robustness_blocks() {
    let mut report = baseline_report();
    report["run"]["req"]["strategy_kind"] = json!("Cdmc");
    report["validation"]["res"]["backend_used"] = json!("Gpu");
    report["validation"]["res"]["backend_route"] = json!("GpuKernel");
    report["validation"]["res"]["backend_fallback_reason"] = serde_json::Value::Null;

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains(
        "CDMC GPU validation uses GpuKernel-backed candidate and slice reruns for the fixed-candidate stack."
    ));
    assert!(xml.contains("GPU-backed reruns/evaluation: base reruns"));
    assert!(xml.contains("CPU statistical/gate reduction: probability metrics + Pareto marking + hard gates + final ordering"));
    assert!(xml.contains("Mixed/GPU-assisted tail: perturb"));
    assert!(xml.contains("CPU auxiliary tail: trade_bootstrap"));
    assert!(xml.contains("perturb (mixed/GPU-assisted, route-dependent)"));
    assert!(xml.contains("CPU auxiliary tail"));
    assert!(xml.contains("WF Efficiency"));
    assert!(xml.contains("Bootstrap Compare"));
    assert!(xml.contains("Regime Flag"));
    assert!(xml.contains("Friction Stability"));
}

#[test]
fn workbook_surfaces_vcet_gpu_backed_expected_execution_truthfully() {
    let mut report = baseline_report();
    report["run"]["req"]["strategy_kind"] = json!("Vcet");
    report["validation"]["res"]["backend_used"] = json!("Gpu");
    report["validation"]["res"]["backend_route"] = json!("GpuKernel");
    report["validation"]["res"]["backend_fallback_reason"] = serde_json::Value::Null;

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains(
        "VCET GPU validation uses GpuKernel-backed base, walk-forward, reopt walk-forward, CSCV-slice, and SPA-slice reruns."
    ));
    assert!(xml.contains("GPU-backed reruns/evaluation: base reruns"));
    assert!(xml.contains("CPU statistical/gate reduction: probability metrics + Pareto marking + hard gates + final ordering"));
    assert!(xml.contains("Mixed/GPU-assisted tail: perturb"));
    assert!(xml.contains("CPU auxiliary tail: trade_bootstrap"));
    assert!(!xml.contains("GPU-native:"));
}

#[test]
fn workbook_surfaces_composite_mo_cpu_route_detail_note_in_expected_execution() {
    let mut report = baseline_report();
    report["run"]["req"]["strategy_kind"] = json!("CompositeMo");
    report["validation"]["req"]["backend"] = json!({ "backend": "GpuKernel", "device_id": 0 });
    report["validation"]["res"]["backend_used"] = json!("Cpu");
    report["validation"]["res"]["backend_route"] = json!("CpuOnly");
    report["validation"]["res"]["backend_fallback_reason"] = json!(
        "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker)."
    );

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains(
        "CompositeMo uses the CPU-preferred validation route; GPU fixed-candidate validation currently follows the same plain CompositeCpu HardGate eligibility contract (uniform strategy/indicator settings, vectorized engine, no inline cross_ticker)."
    ));
}

#[test]
fn workbook_surfaces_plain_composite_cpu_gpu_route_detail_note_in_expected_execution() {
    let mut report = baseline_report();
    report["run"]["req"]["strategy_kind"] = json!("CompositeCpu");
    report["validation"]["req"]["backend"] = json!({ "backend": "GpuKernel", "device_id": 0 });
    report["validation"]["res"]["backend_used"] = json!("Gpu");
    report["validation"]["res"]["backend_route"] = json!("GpuKernel");
    report["validation"]["res"]["backend_fallback_reason"] = serde_json::Value::Null;

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let xml = all_xml(&bytes);
    assert!(xml.contains("Expected Execution"));
    assert!(xml.contains("Expected Categories"));
    assert!(xml.contains("Expected Notes"));
    assert!(xml.contains(
        "Plain CompositeCpu GPU validation currently requires uniform HalfTrend parameters across finalists; mixed finalist sets can fall back to CPU."
    ));
}

#[test]
fn workbook_surfaces_double_ma_plateau_analysis() {
    let report = json!({
      "version": 1,
      "kind": "vectorgrid_report",
      "ts": "2026-03-13T00:00:00Z",
      "run": { "req": { "strategy_kind": "DoubleMa" } },
      "optimization": {
        "res": {
          "top": [
            [
              { "fast_ma_type": "alma", "fast_len": 9, "slow_ma_type": "alma", "slow_len": 30 },
              { "pnl": 1.20, "sharpe": 1.40, "max_dd": 0.18, "trades": 22 }
            ],
            [
              { "fast_ma_type": "alma", "fast_len": 10, "slow_ma_type": "alma", "slow_len": 30 },
              { "pnl": 1.16, "sharpe": 1.34, "max_dd": 0.19, "trades": 21 }
            ],
            [
              { "fast_ma_type": "alma", "fast_len": 9, "slow_ma_type": "alma", "slow_len": 32 },
              { "pnl": 1.14, "sharpe": 1.31, "max_dd": 0.20, "trades": 20 }
            ],
            [
              { "fast_ma_type": "alma", "fast_len": 10, "slow_ma_type": "alma", "slow_len": 32 },
              { "pnl": 1.10, "sharpe": 1.28, "max_dd": 0.21, "trades": 20 }
            ],
            [
              { "fast_ma_type": "alma", "fast_len": 8, "slow_ma_type": "alma", "slow_len": 30 },
              { "pnl": 1.08, "sharpe": 1.26, "max_dd": 0.20, "trades": 23 }
            ],
            [
              { "fast_ma_type": "alma", "fast_len": 14, "slow_ma_type": "alma", "slow_len": 42 },
              { "pnl": 0.70, "sharpe": 0.82, "max_dd": 0.29, "trades": 18 }
            ]
          ]
        }
      },
      "validation": {
        "res": {
          "data_id": "double_ma_plateau",
          "backend_used": "Gpu",
          "backend_route": "GpuKernel",
          "run_verdict": {
            "passed": true,
            "approved": true,
            "candidates_tested": 2,
            "candidates_survived_hard_gates": 1,
            "approved_candidate_count": 1,
            "final_recommendation": "single_candidate_supported",
            "warning_flags": [],
            "fail_reasons": [],
            "first_approved_rank": 1
          },
          "candidates": [
            {
              "params": {
                "fast_ma_type": "alma",
                "fast_len": 9,
                "slow_ma_type": "alma",
                "slow_len": 30
              },
              "in_sample": { "pnl": 1.20, "sharpe": 1.40, "max_dd": 0.18, "trades": 22 },
              "psr": 0.88,
              "dsr": 0.81,
              "track_record_bars": 300,
              "min_track_record_length_bars": 240,
              "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
              "robust_score": 1.52,
              "robust_pareto": true
            },
            {
              "params": {
                "fast_ma_type": "alma",
                "fast_len": 14,
                "slow_ma_type": "alma",
                "slow_len": 42
              },
              "in_sample": { "pnl": 0.70, "sharpe": 0.82, "max_dd": 0.29, "trades": 18 },
              "psr": 0.44,
              "dsr": 0.31,
              "track_record_bars": 210,
              "min_track_record_length_bars": 240,
              "validation_verdict": { "passed": false, "warning_flags": ["min_track_record_length_not_met:210/240"], "fail_reasons": ["psr_below_min:0.50"] },
              "robust_score": 0.74,
              "robust_pareto": false
            }
          ]
        }
      }
    });

    let bytes = build_report_xlsx_bytes_headless(&report, Some(10)).expect("xlsx");
    let titles = chart_titles(&bytes);
    let xml = all_xml(&bytes);
    assert!(xml.contains("Plateau Score"));
    assert!(xml.contains("Plateau Label"));
    assert!(xml.contains("Plateau Src"));
    assert!(xml.contains("Plateau N"));
    assert!(xml.contains("Plateau Sharpe P50"));
    assert!(xml.contains("Plateau DD Worst"));
    assert!(xml.contains("Top Candidate Snapshot"));
    assert!(xml.contains("Plateau"));
    assert!(xml.contains("Strong"));
    assert!(xml.contains("n=5"));
    assert!(xml.contains("top"));
    assert!(titles
        .iter()
        .any(|title| title == "Plateau score by robust rank"));
}
