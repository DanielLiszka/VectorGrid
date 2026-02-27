# Post-Release Telemetry Review

- Generated: 2026-02-26T07:05:35.226Z
- Status: **PASS**
- Checks passed: 17/17

## Sources

- Baseline: `scripts/baselines/workflow_regression_baseline.json`
- Summary: `target/workflow_regression_summary.json`
- UI metrics: `scripts/baselines/ui_capture/perf_metrics.json`
- Memory metrics: `scripts/baselines/ui_capture/optimization_memory_metrics.json`
- Samples manifest: `scripts/baselines/samples/manifest.json`

## Check Results

- [PASS] `workflow_runtime_not_regressed_50pct` - runtime_ms=384 baseline=345
- [PASS] `robust_score_p10_non_decreasing` - robust_score_p10 observed=0 baseline=0
- [PASS] `holdout_score_p10_non_decreasing` - holdout_score_p10 observed=0 baseline=0
- [PASS] `max_dd_p90_non_increasing` - max_dd_p90 observed=0 baseline=0
- [PASS] `holdout_max_dd_p90_non_increasing` - holdout_max_dd_p90 observed=0 baseline=0
- [PASS] `ui_cold_start_under_3000ms` - app_cold_start_ms=40
- [PASS] `ui_tti_under_10000ms` - time_to_interactive_ms=566
- [PASS] `ui_live_bar_avg_under_2ms` - avg_ms_per_event=0.002 events=200
- [PASS] `ui_live_bar_sample_size_at_least_100` - events=200
- [PASS] `optimization_peak_working_set_under_1024mb` - peak_working_set_mb=39.38
- [PASS] `optimization_peak_private_under_1024mb` - peak_private_mb=33.55
- [PASS] `samples_output_present:top_rows_csv` - top_rows_csv=scripts/baselines/samples/top_rows.csv
- [PASS] `samples_output_present:validation_shortlist_csv` - validation_shortlist_csv=scripts/baselines/samples/validation_shortlist.csv
- [PASS] `samples_output_present:run_artifact_json` - run_artifact_json=scripts/baselines/samples/run_artifact.json
- [PASS] `samples_output_present:batch_summary_json` - batch_summary_json=scripts/baselines/samples/batch_summary.json
- [PASS] `samples_output_present:report_json` - report_json=scripts/baselines/samples/report.json
- [PASS] `samples_output_present:report_xlsx` - report_xlsx=scripts/baselines/samples/report.xlsx
