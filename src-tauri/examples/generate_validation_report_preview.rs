use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;
use ta_desktop_demo_app::commands::build_report_xlsx_bytes_headless;

fn sample_report() -> serde_json::Value {
    json!({
      "version": 1,
      "kind": "vectorgrid_report",
      "ts": "2026-03-13T12:00:00Z",
      "run": {
        "req": {
          "strategy_kind": "Asr"
        }
      },
      "validation": {
        "req": {
          "min_psr": 0.50,
          "min_dsr": 0.20,
          "min_cscv_oos_rank_frac": 0.55,
          "max_pbo": 0.30
        },
        "res": {
          "data_id": "BTCUSD_1h_preview",
          "runtime_ms": 1823,
          "backend_used": "Cpu",
          "backend_route": "Auto->CpuOnly",
          "backend_fallback_reason": "GPU validation runtime not implemented yet; using CPU validation.",
          "run_verdict": {
            "passed": true,
            "approved": true,
            "candidates_tested": 6,
            "candidates_survived_hard_gates": 3,
            "approved_candidate_count": 2,
            "final_recommendation": "approved_with_warnings",
            "warning_flags": ["high_pbo"],
            "fail_reasons": [],
            "first_approved_rank": 2,
            "top_candidate_track_record_bars": 220,
            "top_candidate_min_track_record_length_bars": 180,
            "top_candidate_psr": 0.81,
            "top_candidate_dsr": 0.72,
            "pbo": 0.28,
            "max_pbo": 0.30
          },
          "spa": {
            "scope": "validation_search",
            "candidate_count": 6,
            "slice_count": 12,
            "bootstrap_samples": 500,
            "best_candidate_key": "ema_12_36",
            "best_candidate_mean_diff": 0.11,
            "best_candidate_t_stat": 2.4,
            "p_value": 0.07,
            "passed": true
          },
          "reopt_walk_forward": {
            "windows": 9,
            "top_n_candidates": 5,
            "unique_winners": 3,
            "winner_changes": 2,
            "dominant_winner_frac": 0.56,
            "winner_param_drift_mean": 0.18,
            "winner_param_drift_max": 0.45,
            "train_score_median": 1.02,
            "forward_positive_pnl_frac": 0.67,
            "forward_sharpe_median": 1.11,
            "forward_sharpe_worst": 0.44,
            "forward_max_dd_median": 0.19,
            "forward_max_dd_worst": 0.31,
            "forward_pnl_median": 0.23,
            "forward_pnl_worst": -0.04,
            "forward_trades_median": 14,
            "forward_trades_worst": 7,
            "window_pnl_efficiency_median": 0.84,
            "window_pnl_efficiency_worst": -0.18,
            "window_sharpe_efficiency_median": 0.77,
            "window_sharpe_efficiency_worst": 0.28,
            "window_sharpe_efficiency_std": 0.21
          },
          "cross_ticker": {
            "datasets": [
              { "label": "ETHUSD", "csv_path": "C:/data/eth.csv", "bars": 1200 },
              { "label": "SOLUSD", "csv_path": "C:/data/sol.csv", "bars": 1100 }
            ]
          },
          "cscv_pbo": {
            "slices": 12,
            "slice_bars": 240,
            "candidates": 6,
            "combinations": 924,
            "combinations_possible": 924,
            "pbo": 0.28,
            "lambda_p25": -0.2,
            "lambda_p50": 0.1,
            "lambda_p75": 0.4,
            "degradation_slope": -0.1,
            "degradation_intercept": 0.2,
            "degradation_r2": 0.64,
            "threshold": 0.0,
            "prob_oos_below_threshold": 0.33
          },
          "candidates": [
            {
              "params": {
                "fast_ma_type": "ema",
                "fast_len": 12,
                "slow_ma_type": "sma",
                "slow_len": 36,
                "asr": { "angle_deg_min": 10, "z_entry": 2.0 }
              },
              "in_sample": { "pnl": 0.52, "sharpe": 1.64, "max_dd": 0.18, "trades": 17, "exposure": 0.61 },
              "walk_forward": { "sharpe_median": 1.11, "max_dd_worst": 0.31 },
              "trade_bootstrap": { "prob_pnl_positive": 0.77, "pnl_p50": 0.21, "max_dd_p95": 0.39 },
              "cross_ticker": { "score": 0.42 },
              "cross_ticker_pareto": true,
              "cscv_pbo": { "is_selected_frac": 0.75, "oos_rank_frac_p50": 0.71, "oos_metric_p50": 0.18 },
              "perturb": { "pnl_mean": 0.16, "pnl_worst": -0.07, "pnl_positive_frac": 0.67 },
              "psr": 0.81,
              "dsr": 0.72,
              "track_record_bars": 220,
              "min_track_record_length_bars": 180,
              "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
              "robust_score": 1.41,
              "robust_pareto": true
            },
            {
              "params": {
                "fast_ma_type": "ema",
                "fast_len": 10,
                "slow_ma_type": "sma",
                "slow_len": 44,
                "asr": { "angle_deg_min": 8, "z_entry": 1.8 }
              },
              "in_sample": { "pnl": 0.43, "sharpe": 1.34, "max_dd": 0.21, "trades": 19, "exposure": 0.57 },
              "walk_forward": { "sharpe_median": 1.03, "max_dd_worst": 0.29 },
              "trade_bootstrap": { "prob_pnl_positive": 0.71, "pnl_p50": 0.16, "max_dd_p95": 0.43 },
              "cross_ticker": { "score": 0.39 },
              "cross_ticker_pareto": true,
              "cscv_pbo": { "is_selected_frac": 0.66, "oos_rank_frac_p50": 0.68, "oos_metric_p50": 0.12 },
              "perturb": { "pnl_mean": 0.13, "pnl_worst": -0.09, "pnl_positive_frac": 0.61 },
              "psr": 0.74,
              "dsr": 0.66,
              "track_record_bars": 240,
              "min_track_record_length_bars": 210,
              "validation_verdict": { "passed": true, "warning_flags": ["high_turnover"], "fail_reasons": [] },
              "robust_score": 1.22,
              "robust_pareto": true
            },
            {
              "params": {
                "fast_ma_type": "sma",
                "fast_len": 18,
                "slow_ma_type": "ema",
                "slow_len": 54,
                "mkcs": { "compression_threshold": 0.01, "reexpansion_threshold": 0.03 }
              },
              "in_sample": { "pnl": 0.18, "sharpe": 0.72, "max_dd": 0.33, "trades": 12, "exposure": 0.44 },
              "walk_forward": { "sharpe_median": 0.48, "max_dd_worst": 0.41 },
              "trade_bootstrap": { "prob_pnl_positive": 0.55, "pnl_p50": 0.06, "max_dd_p95": 0.52 },
              "cross_ticker": { "score": 0.11 },
              "cross_ticker_pareto": false,
              "cscv_pbo": { "is_selected_frac": 0.25, "oos_rank_frac_p50": 0.44, "oos_metric_p50": -0.03 },
              "perturb": { "pnl_mean": -0.02, "pnl_worst": -0.19, "pnl_positive_frac": 0.31 },
              "psr": 0.43,
              "dsr": 0.39,
              "track_record_bars": 140,
              "min_track_record_length_bars": 240,
              "validation_verdict": { "passed": false, "warning_flags": ["min_track_record_length_not_met:140/240"], "fail_reasons": ["psr_below_min:0.50"] },
              "robust_score": 0.29,
              "robust_pareto": false
            }
          ]
        }
      }
    })
}

fn output_path() -> PathBuf {
    if let Some(path) = env::args_os().nth(1) {
        return PathBuf::from(path);
    }
    PathBuf::from("research").join("validation_report_preview.xlsx")
}

fn main() {
    let report = sample_report();
    let bytes =
        build_report_xlsx_bytes_headless(&report, Some(25)).expect("build validation xlsx preview");
    let path = output_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create preview dir");
    }
    fs::write(&path, bytes).expect("write preview xlsx");
    println!("{}", path.display());
}
