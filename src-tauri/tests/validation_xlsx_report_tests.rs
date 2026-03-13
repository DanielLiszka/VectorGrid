use serde_json::json;
use std::io::{Cursor, Read};
use ta_desktop_demo_app::commands::build_report_xlsx_bytes_headless;
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

fn row_count(sheet_xml: &str) -> usize {
    sheet_xml.matches("<row ").count()
}

fn baseline_report() -> serde_json::Value {
    json!({
      "version": 1,
      "kind": "vectorbt_report",
      "ts": "2026-03-13T00:00:00Z",
      "run": { "req": { "strategy_kind": "CompositeTrend" } },
      "validation": {
        "req": {
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
            "forward_trades_worst": 5
          },
          "cross_ticker": {
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
              "walk_forward": { "sharpe_median": 1.34, "max_dd_worst": 0.29 },
              "trade_bootstrap": { "prob_pnl_positive": 0.81, "pnl_p50": 0.22, "max_dd_p95": 0.41 },
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
              "walk_forward": { "sharpe_median": 1.01, "max_dd_worst": 0.31 },
              "trade_bootstrap": { "prob_pnl_positive": 0.72, "pnl_p50": 0.17, "max_dd_p95": 0.43 },
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
fn workbook_contains_expected_sheets_charts_and_dashboard_content() {
    let bytes = build_report_xlsx_bytes_headless(&baseline_report(), Some(3)).expect("xlsx");
    assert_eq!(&bytes[0..2], b"PK");

    let workbook_xml = read_entry(&bytes, "xl/workbook.xml");
    assert!(workbook_xml.contains("Approved Candidates"));
    assert!(workbook_xml.contains("Shortlist"));
    assert!(workbook_xml.contains("Distributions"));
    assert!(workbook_xml.contains("Audit Data"));
    assert!(workbook_xml.contains("Dashboard"));
    assert!(workbook_xml.contains("activeTab=\"4\""));

    let names: Vec<String> = xml_entries(&bytes)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| name.starts_with("xl/charts/chart"))
        .collect();
    assert!(names.len() >= 6, "expected dashboard and distribution charts, got {:?}", names);
    let titles = chart_titles(&bytes);
    assert!(titles.iter().any(|title| title == "Approval state counts"));
    assert!(titles.iter().any(|title| title == "Top rejection / review reasons"));
    assert!(titles.iter().any(|title| title == "Profile counts"));
    assert!(titles.iter().any(|title| title == "Robust score distribution by rank"));
    assert!(titles.iter().any(|title| title == "Sharpe vs MaxDD"));

    let approved_sheet = read_entry(&bytes, "xl/worksheets/sheet1.xml");
    let shortlist_sheet = read_entry(&bytes, "xl/worksheets/sheet2.xml");
    assert_eq!(row_count(&approved_sheet), 3, "header + 2 approved rows");
    assert_eq!(row_count(&shortlist_sheet), 4, "header + 3 shortlist rows");

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
    assert!(xml.contains("Approval State"));
    assert!(xml.contains("Top Rejection / Review Reason"));
    assert!(xml.contains("Approval state counts"));
    assert!(xml.contains("Approved"));
    assert!(xml.contains("Rejected"));
    assert!(xml.contains("psr_below_min:0.50"));
    assert!(xml.contains("high_turnover"));
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
    assert_eq!(row_count(&approved_sheet), 1, "approved sheet should only have header");

    let xml = all_xml(&bytes);
    assert!(xml.contains("spa_not_significant"));
    assert!(xml.contains("run verdict: spa_not_significant"));
}

#[test]
fn workbook_supports_legacy_compatibility_mode() {
    let report = json!({
      "version": 1,
      "kind": "vectorbt_report",
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
    assert!(xml.contains("Compatibility mode: workbook generated from a legacy validation artifact"));
    assert!(xml.contains("legacy shortlist compatibility mode"));
    assert!(xml.contains("not run"));
}

#[test]
fn workbook_supports_all_profile_families_in_strategy_agnostic_mode() {
    let profiles = [
        "asr", "vcc", "vemr", "rocb", "mkcs", "kqsr", "carm", "kvdf", "cdmc",
    ];

    for profile in profiles {
        let report = json!({
          "version": 1,
          "kind": "vectorbt_report",
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
