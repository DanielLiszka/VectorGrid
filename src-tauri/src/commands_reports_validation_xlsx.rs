use super::commands_reports_validation_common::{
    build_model_from_report, ExportCandidateRow, ValidationExportModel,
};
use rust_xlsxwriter::{
    Chart, ChartType, Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError,
};
use serde_json::Value;

const COL_RANK: u16 = 0;
const COL_APPROVAL: u16 = 1;
const COL_HARD_GATE: u16 = 2;
const COL_PROFILE: u16 = 3;
const COL_PARAMS_SUMMARY: u16 = 4;
const COL_ROBUST: u16 = 5;
const COL_PSR: u16 = 6;
const COL_DSR: u16 = 7;
const COL_TRACK_BARS: u16 = 8;
const COL_MINTRL: u16 = 9;
const COL_PNL: u16 = 10;
const COL_SHARPE: u16 = 11;
const COL_MAX_DD: u16 = 12;
const COL_TRADES: u16 = 13;
const COL_WF_SHARPE: u16 = 14;
const COL_WF_DD: u16 = 15;
const COL_CSCV_RANK: u16 = 16;
const COL_CSCV_METRIC: u16 = 17;
const COL_CSCV_SEL: u16 = 18;
const COL_CROSS_SCORE: u16 = 19;
const COL_CROSS_PARETO: u16 = 20;
const COL_BOOTSTRAP_PROB: u16 = 21;
const COL_PERTURB_MEAN: u16 = 22;
const COL_WARNINGS: u16 = 23;
const COL_FAIL_REASONS: u16 = 24;
const COL_PARAMS_JSON: u16 = 25;

fn write_optional_number(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<f64>,
    fmt: &Format,
) -> Result<(), String> {
    match value {
        Some(v) => sheet
            .write_number_with_format(row, col, v, fmt)
            .map_err(|e: XlsxError| e.to_string())?,
        None => sheet
            .write_string_with_format(row, col, "not run", fmt)
            .map_err(|e: XlsxError| e.to_string())?,
    };
    Ok(())
}

fn write_optional_usize(
    sheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<usize>,
    fmt: &Format,
) -> Result<(), String> {
    match value {
        Some(v) => sheet
            .write_number_with_format(row, col, v as f64, fmt)
            .map_err(|e: XlsxError| e.to_string())?,
        None => sheet
            .write_string_with_format(row, col, "n/a", fmt)
            .map_err(|e: XlsxError| e.to_string())?,
    };
    Ok(())
}

fn display_approval_label(label: &str) -> &'static str {
    match label {
        "approved" => "Approved",
        "rejected" => "Rejected",
        "review" => "Review",
        "not_approved" => "Not Approved",
        "compat_shortlist" => "Compat Shortlist",
        "compat_context" => "Compat Context",
        _ => "Review",
    }
}

fn approval_label_counts(model: &ValidationExportModel) -> Vec<(String, usize)> {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &model.all_rows {
        *counts.entry(row.approval_label.clone()).or_insert(0) += 1;
    }

    let mut out = Vec::new();
    for label in [
        "approved",
        "review",
        "rejected",
        "not_approved",
        "compat_shortlist",
        "compat_context",
    ] {
        if let Some(count) = counts.remove(label) {
            out.push((display_approval_label(label).to_string(), count));
        }
    }
    for (label, count) in counts {
        out.push((label, count));
    }
    out
}

fn top_reason_counts(model: &ValidationExportModel, limit: usize) -> Vec<(String, usize)> {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &model.all_rows {
        if row.final_approved || row.approval_reason.trim().is_empty() || row.approval_reason == "approved" {
            continue;
        }
        *counts.entry(row.approval_reason.clone()).or_insert(0) += 1;
    }
    let mut out: Vec<(String, usize)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out.truncate(limit);
    out
}

fn set_candidate_sheet_layout(sheet: &mut Worksheet) {
    sheet.set_zoom(110);
    let _ = sheet.set_freeze_panes(1, 0);
    let _ = sheet.set_column_width(COL_RANK, 8);
    let _ = sheet.set_column_width(COL_APPROVAL, 18);
    let _ = sheet.set_column_width(COL_HARD_GATE, 12);
    let _ = sheet.set_column_width(COL_PROFILE, 16);
    let _ = sheet.set_column_width(COL_PARAMS_SUMMARY, 38);
    let _ = sheet.set_column_width(COL_ROBUST, 12);
    let _ = sheet.set_column_width(COL_PSR, 10);
    let _ = sheet.set_column_width(COL_DSR, 10);
    let _ = sheet.set_column_width(COL_TRACK_BARS, 11);
    let _ = sheet.set_column_width(COL_MINTRL, 11);
    let _ = sheet.set_column_width(COL_PNL, 12);
    let _ = sheet.set_column_width(COL_SHARPE, 12);
    let _ = sheet.set_column_width(COL_MAX_DD, 12);
    let _ = sheet.set_column_width(COL_TRADES, 10);
    let _ = sheet.set_column_width(COL_WF_SHARPE, 12);
    let _ = sheet.set_column_width(COL_WF_DD, 12);
    let _ = sheet.set_column_width(COL_CSCV_RANK, 12);
    let _ = sheet.set_column_width(COL_CSCV_METRIC, 12);
    let _ = sheet.set_column_width(COL_CSCV_SEL, 12);
    let _ = sheet.set_column_width(COL_CROSS_SCORE, 12);
    let _ = sheet.set_column_width(COL_CROSS_PARETO, 12);
    let _ = sheet.set_column_width(COL_BOOTSTRAP_PROB, 12);
    let _ = sheet.set_column_width(COL_PERTURB_MEAN, 12);
    let _ = sheet.set_column_width(COL_WARNINGS, 28);
    let _ = sheet.set_column_width(COL_FAIL_REASONS, 30);
    let _ = sheet.set_column_width(COL_PARAMS_JSON, 44);
}

fn write_candidate_headers(sheet: &mut Worksheet, header_fmt: &Format) -> Result<(), String> {
    let headers = [
        "Rank",
        "Approval",
        "HardGate",
        "Profile",
        "Params Summary",
        "Robust",
        "PSR",
        "DSR",
        "TrackBars",
        "MinTRL",
        "PnL",
        "Sharpe",
        "MaxDD",
        "Trades",
        "WF Sharpe",
        "WF DD Worst",
        "CSCV Rank",
        "CSCV Metric",
        "CSCV SelFrac",
        "Cross Score",
        "Cross Pareto",
        "Bootstrap Prob+",
        "Perturb Mean",
        "Warnings",
        "Fail Reasons",
        "Params JSON",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn write_candidate_rows(
    sheet: &mut Worksheet,
    rows: &[ExportCandidateRow],
    text_fmt: &Format,
    mono_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
    compat_fmt: &Format,
) -> Result<(), String> {
    for (idx, row) in rows.iter().enumerate() {
        let r = idx as u32 + 1;
        sheet
            .write_number_with_format(r, COL_RANK, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        let approval_fmt_ref = match row.approval_label.as_str() {
            "approved" => approved_fmt,
            "rejected" => rejected_fmt,
            "compat_shortlist" | "compat_context" => compat_fmt,
            _ => review_fmt,
        };
        sheet
            .write_string_with_format(
                r,
                COL_APPROVAL,
                display_approval_label(&row.approval_label),
                approval_fmt_ref,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_HARD_GATE,
                if row.hard_gate_passed == Some(true) {
                    "pass"
                } else if row.hard_gate_passed == Some(false) {
                    "fail"
                } else {
                    "compat"
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, COL_PROFILE, &row.profile_kind, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, COL_PARAMS_SUMMARY, &row.params_summary, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_ROBUST, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, COL_PSR, row.psr, pct_fmt)?;
        write_optional_number(sheet, r, COL_DSR, row.dsr, pct_fmt)?;
        write_optional_usize(sheet, r, COL_TRACK_BARS, row.track_record_bars, int_fmt)?;
        write_optional_usize(sheet, r, COL_MINTRL, row.min_track_record_length_bars, int_fmt)?;
        sheet
            .write_number_with_format(r, COL_PNL, row.pnl, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_SHARPE, row.sharpe, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_MAX_DD, row.max_dd, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_TRADES, row.trades, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, COL_WF_SHARPE, row.wf_sharpe_median, num_fmt)?;
        write_optional_number(sheet, r, COL_WF_DD, row.wf_dd_worst, num_fmt)?;
        write_optional_number(sheet, r, COL_CSCV_RANK, row.cscv_oos_rank_frac_p50, pct_fmt)?;
        write_optional_number(sheet, r, COL_CSCV_METRIC, row.cscv_oos_metric_p50, num_fmt)?;
        write_optional_number(sheet, r, COL_CSCV_SEL, row.cscv_selected_frac, pct_fmt)?;
        write_optional_number(sheet, r, COL_CROSS_SCORE, row.cross_score, num_fmt)?;
        sheet
            .write_string_with_format(
                r,
                COL_CROSS_PARETO,
                if row.cross_pareto { "yes" } else { "no" },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(
            sheet,
            r,
            COL_BOOTSTRAP_PROB,
            row.bootstrap_prob_pnl_positive,
            pct_fmt,
        )?;
        write_optional_number(sheet, r, COL_PERTURB_MEAN, row.perturb_pnl_mean, num_fmt)?;
        sheet
            .write_string_with_format(r, COL_WARNINGS, &row.warning_flags, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, COL_FAIL_REASONS, &row.fail_reasons, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, COL_PARAMS_JSON, &row.params_json, mono_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn write_candidate_sheet(
    workbook: &mut Workbook,
    name: &str,
    rows: &[ExportCandidateRow],
    title_note: &str,
    header_fmt: &Format,
    text_fmt: &Format,
    mono_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
    compat_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet.set_name(name).map_err(|e: XlsxError| e.to_string())?;
    set_candidate_sheet_layout(sheet);
    write_candidate_headers(sheet, header_fmt)?;
    write_candidate_rows(
        sheet,
        rows,
        text_fmt,
        mono_fmt,
        num_fmt,
        pct_fmt,
        int_fmt,
        approved_fmt,
        review_fmt,
        rejected_fmt,
        compat_fmt,
    )?;
    if !title_note.is_empty() {
        sheet
            .write_string_with_format(0, COL_PARAMS_JSON + 1, title_note, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn build_dashboard(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    label_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
    compat_fmt: &Format,
) -> Result<(), String> {
    let dashboard = workbook.add_worksheet();
    dashboard
        .set_name("Dashboard")
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard.set_active(true);
    dashboard.set_zoom(115);
    let _ = dashboard.set_column_width(0, 24);
    let _ = dashboard.set_column_width(1, 34);
    let _ = dashboard.set_column_width(2, 22);
    let _ = dashboard.set_column_width(3, 20);
    let _ = dashboard.set_column_width(4, 18);
    let _ = dashboard.set_column_width(5, 18);
    let _ = dashboard.set_column_width(6, 18);
    let _ = dashboard.set_column_width(7, 18);

    dashboard
        .merge_range(0, 0, 0, 7, "Validation XLSX Report", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if model.compatibility_mode {
        dashboard
            .merge_range(
                1,
                0,
                1,
                7,
                "Compatibility mode: workbook generated from a legacy validation artifact without the full verdict contract.",
                compat_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let mut meta_row = if model.compatibility_mode { 3 } else { 2 };
    let recommendation = model
        .run_verdict
        .as_ref()
        .map(|verdict| verdict.final_recommendation.as_str())
        .unwrap_or("legacy_compatibility_mode");
    let runtime_text = model
        .runtime_ms
        .map(|value| format!("{value} ms"))
        .unwrap_or_else(|| "-".to_string());
    for (label, value) in [
        ("Timestamp", model.ts.as_str()),
        ("Strategy", model.strategy_kind.as_str()),
        ("Dataset", model.data_id.as_str()),
        ("Runtime", runtime_text.as_str()),
        ("Recommendation", recommendation),
        (
            "Backend",
            if model.backend_used.is_empty() {
                "unknown"
            } else {
                model.backend_used.as_str()
            },
        ),
        (
            "Route",
            if model.backend_route.is_empty() {
                "-"
            } else {
                model.backend_route.as_str()
            },
        ),
        (
            "Fallback",
            if model.backend_fallback_reason.is_empty() {
                "-"
            } else {
                model.backend_fallback_reason.as_str()
            },
        ),
    ] {
        dashboard
            .write_string_with_format(meta_row, 0, label, label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(meta_row, 1, value, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        meta_row += 1;
    }

    let run_verdict = model.run_verdict.as_ref();
    let approved_count = run_verdict
        .map(|value| value.approved_candidate_count)
        .unwrap_or_else(|| model.approved_rows.len());
    let first_approved_rank = run_verdict.and_then(|value| value.first_approved_rank);
    let candidates_tested = run_verdict
        .map(|value| value.candidates_tested)
        .unwrap_or_else(|| model.all_rows.len());
    let survived_hard_gates = run_verdict
        .map(|value| value.candidates_survived_hard_gates)
        .unwrap_or_else(|| {
            model
                .all_rows
                .iter()
                .filter(|row| row.hard_gate_passed == Some(true))
                .count()
        });
    let run_state = if run_verdict.map(|value| value.passed).unwrap_or(false) {
        "pass"
    } else if run_verdict.is_some() {
        "fail"
    } else {
        "compat"
    };

    dashboard
        .write_string_with_format(2, 3, "Run Verdict", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            2,
            4,
            run_state,
            match run_state {
                "pass" => approved_fmt,
                "fail" => rejected_fmt,
                _ => compat_fmt,
            },
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(3, 3, "Approved", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_number_with_format(3, 4, approved_count as f64, int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(4, 3, "Hard Gates", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            4,
            4,
            &format!("{survived_hard_gates}/{candidates_tested}"),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(5, 3, "First Approved Rank", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(rank) = first_approved_rank {
        dashboard
            .write_number_with_format(5, 4, rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(5, 4, "-", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    if let Some(verdict) = run_verdict {
        dashboard
            .write_string_with_format(6, 3, "Run Warnings", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                6,
                4,
                if verdict.warning_flags.is_empty() {
                    "-"
                } else {
                    verdict.warning_flags.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(7, 3, "Run Fail Reasons", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                7,
                4,
                if verdict.fail_reasons.is_empty() {
                    "-"
                } else {
                    verdict.fail_reasons.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(8, 3, "Run PBO", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 8, 4, verdict.pbo, num_fmt)?;
        dashboard
            .write_string_with_format(9, 3, "Max PBO", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 9, 4, verdict.max_pbo, num_fmt)?;
        dashboard
            .write_string_with_format(10, 3, "Top Candidate PSR", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 10, 4, verdict.top_candidate_psr, pct_fmt)?;
        dashboard
            .write_string_with_format(11, 3, "Top Candidate DSR", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 11, 4, verdict.top_candidate_dsr, pct_fmt)?;
        dashboard
            .write_string_with_format(12, 3, "Track Record", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                12,
                4,
                &match (
                    verdict.top_candidate_track_record_bars,
                    verdict.top_candidate_min_track_record_length_bars,
                ) {
                    (Some(obs), Some(mintrl)) => format!("{obs}/{mintrl}"),
                    _ => "-".to_string(),
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(2, 5, "CSCV / PBO", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(cscv) = model.cscv_pbo.as_ref() {
        dashboard
            .write_string_with_format(3, 5, "Slices/Combos", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                3,
                6,
                &format!("{}/{}", cscv.slices, cscv.combinations),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(4, 5, "PBO", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 4, 6, cscv.pbo, num_fmt)?;
        dashboard
            .write_string_with_format(5, 5, "Slice Bars/Cands", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                5,
                6,
                &format!("{}/{}", cscv.slice_bars, cscv.candidates),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(6, 5, "Prob OOS<thr", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 6, 6, cscv.prob_oos_below_threshold, pct_fmt)?;
        dashboard
            .write_string_with_format(7, 5, "Threshold", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 7, 6, cscv.threshold, num_fmt)?;
        dashboard
            .write_string_with_format(8, 5, "CSCV Status", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                8,
                6,
                if cscv.skipped_reason.is_empty() {
                    "completed"
                } else {
                    cscv.skipped_reason.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(3, 6, "not run", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(9, 5, "SPA", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(spa) = model.spa.as_ref() {
        dashboard
            .write_string_with_format(10, 5, "p-value", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 10, 6, spa.p_value, num_fmt)?;
        dashboard
            .write_string_with_format(11, 5, "Scope/Slices", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                11,
                6,
                &format!("{} / {}", spa.scope, spa.slice_count),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(12, 5, "Cands/Boot", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                12,
                6,
                &format!("{}/{}", spa.candidate_count, spa.bootstrap_samples),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(13, 5, "SPA Status", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                13,
                6,
                if !spa.skipped_reason.is_empty() {
                    spa.skipped_reason.as_str()
                } else if spa.passed == Some(true) {
                    "pass"
                } else if spa.passed == Some(false) {
                    "fail"
                } else {
                    "not run"
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(10, 6, "not run", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(14, 5, "Reopt WF", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(reopt) = model.reopt_walk_forward.as_ref() {
        dashboard
            .write_string_with_format(15, 5, "Windows", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(15, 6, reopt.windows as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(16, 5, "TopN / Unique", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                16,
                6,
                &format!("{}/{}", reopt.top_n_candidates, reopt.unique_winners),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(17, 5, "Winner Changes", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(17, 6, reopt.winner_changes as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(18, 5, "Dominant Winner", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 18, 6, reopt.dominant_winner_frac, pct_fmt)?;
        dashboard
            .write_string_with_format(19, 5, "Fwd Sharpe", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 19, 6, reopt.forward_sharpe_median, num_fmt)?;
        dashboard
            .write_string_with_format(20, 5, "Fwd MaxDD", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 20, 6, reopt.forward_max_dd_worst, num_fmt)?;
        dashboard
            .write_string_with_format(21, 5, "Fwd PnL+", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 21, 6, reopt.forward_positive_pnl_frac, pct_fmt)?;
    } else {
        dashboard
            .write_string_with_format(15, 6, "not run", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(23, 5, "Cross-Ticker", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(cross) = model.cross_ticker.as_ref() {
        dashboard
            .write_string_with_format(24, 5, "Datasets", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(24, 6, cross.dataset_count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(25, 5, "Labels", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                25,
                6,
                if cross.dataset_labels.is_empty() {
                    "-"
                } else {
                    cross.dataset_labels.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(24, 6, "not run", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(23, 3, "Gate Config", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (offset, label, value, is_pct) in [
        (1u32, "Min PSR", model.min_psr, true),
        (2u32, "Min DSR", model.min_dsr, true),
        (3u32, "Min CSCV Rank", model.min_cscv_oos_rank_frac, true),
        (4u32, "Min CSCV Metric", model.min_cscv_oos_metric, false),
        (5u32, "Max PBO", model.max_pbo, true),
    ] {
        let row = 23 + offset;
        dashboard
            .write_string_with_format(row, 3, label, label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        if let Some(v) = value {
            dashboard
                .write_number_with_format(row, 4, v, if is_pct { pct_fmt } else { num_fmt })
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            dashboard
                .write_string_with_format(row, 4, "off", text_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    dashboard
        .write_string_with_format(13, 0, "Sheet Links", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dashboard.write_url_with_text(13, 1, "internal:Approved Candidates!A1", "Approved");
    let _ = dashboard.write_url_with_text(13, 2, "internal:Shortlist!A1", "Shortlist");
    let _ = dashboard.write_url_with_text(13, 3, "internal:Distributions!A1", "Distributions");
    let _ = dashboard.write_url_with_text(13, 4, "internal:Audit Data!A1", "Audit Data");

    dashboard
        .write_string_with_format(15, 0, "Top Candidate Snapshot", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let top_candidate = model
        .approved_rows
        .first()
        .or_else(|| model.shortlist_rows.first())
        .or_else(|| model.all_rows.first());
    if let Some(row) = top_candidate {
        for (offset, (label, value)) in [
            ("Approval", display_approval_label(&row.approval_label)),
            ("Reason", row.approval_reason.as_str()),
            ("Profile", row.profile_kind.as_str()),
            ("Params", row.params_summary.as_str()),
        ]
        .iter()
        .enumerate()
        {
            let base_row = 16 + offset as u32;
            dashboard
                .write_string_with_format(base_row, 0, *label, label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            dashboard
                .write_string_with_format(base_row, 1, *value, text_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        for (offset, (label, value, pct_metric)) in [
            ("Robust", Some(row.robust_score), false),
            ("PSR", row.psr, true),
            ("DSR", row.dsr, true),
            ("WF Sharpe", row.wf_sharpe_median, false),
            ("Cross Score", row.cross_score, false),
            ("Bootstrap Prob+", row.bootstrap_prob_pnl_positive, true),
        ]
        .iter()
        .enumerate()
        {
            let base_row = 16 + offset as u32;
            dashboard
                .write_string_with_format(base_row, 3, *label, label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            write_optional_number(
                dashboard,
                base_row,
                4,
                *value,
                if *pct_metric { pct_fmt } else { num_fmt },
            )?;
        }
    } else {
        dashboard
            .write_string_with_format(16, 1, "No candidates available.", rejected_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(30, 0, "Top Visible Rows", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (col, header) in [
        "Rank", "Approval", "Profile", "Params", "Robust", "PSR", "DSR", "Reason",
    ]
    .iter()
    .enumerate()
    {
        dashboard
            .write_string_with_format(31, col as u16, *header, label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let mut table_rows = Vec::new();
    let mut seen_ranks = std::collections::BTreeSet::new();
    for row in &model.approved_rows {
        if seen_ranks.insert(row.rank) {
            table_rows.push(row);
        }
        if table_rows.len() >= 8 {
            break;
        }
    }
    if table_rows.len() < 8 {
        for row in &model.shortlist_rows {
            if seen_ranks.insert(row.rank) {
                table_rows.push(row);
            }
            if table_rows.len() >= 8 {
                break;
            }
        }
    }
    if table_rows.len() < 8 {
        for row in &model.all_rows {
            if seen_ranks.insert(row.rank) {
                table_rows.push(row);
            }
            if table_rows.len() >= 8 {
                break;
            }
        }
    }
    for (idx, row) in table_rows.iter().enumerate() {
        let r = 32 + idx as u32;
        let status_fmt = match row.approval_label.as_str() {
            "approved" => approved_fmt,
            "rejected" => rejected_fmt,
            "compat_shortlist" | "compat_context" => compat_fmt,
            _ => review_fmt,
        };
        dashboard
            .write_number_with_format(r, 0, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(r, 1, display_approval_label(&row.approval_label), status_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(r, 2, &row.profile_kind, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(r, 3, &row.params_summary, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(r, 4, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, r, 5, row.psr, pct_fmt)?;
        write_optional_number(dashboard, r, 6, row.dsr, pct_fmt)?;
        dashboard
            .write_string_with_format(r, 7, &row.approval_reason, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    if !model.all_rows.is_empty() {
        let row_min = 1u32;
        let row_max = model.all_rows.len() as u32;
        let mut score_chart = Chart::new(ChartType::Column);
        score_chart
            .add_series()
            .set_categories(("Audit Data", row_min, COL_RANK, row_max, COL_RANK))
            .set_values(("Audit Data", row_min, COL_ROBUST, row_max, COL_ROBUST));
        score_chart.title().set_name("Robust score by rank");
        score_chart.legend().set_hidden();
        dashboard
            .insert_chart_with_offset(2, 9, &score_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;

        let mut rr_chart = Chart::new(ChartType::Scatter);
        rr_chart
            .add_series()
            .set_categories(("Audit Data", row_min, COL_MAX_DD, row_max, COL_MAX_DD))
            .set_values(("Audit Data", row_min, COL_SHARPE, row_max, COL_SHARPE));
        rr_chart.title().set_name("Sharpe vs MaxDD");
        rr_chart.x_axis().set_name("MaxDD");
        rr_chart.y_axis().set_name("Sharpe");
        rr_chart.legend().set_hidden();
        dashboard
            .insert_chart_with_offset(20, 9, &rr_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

fn build_distributions(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    int_fmt: &Format,
) -> Result<(), String> {
    use std::collections::BTreeMap;

    let dist = workbook.add_worksheet();
    dist.set_name("Distributions")
        .map_err(|e: XlsxError| e.to_string())?;
    dist.set_zoom(110);
    let _ = dist.set_column_width(0, 24);
    let _ = dist.set_column_width(1, 12);
    let _ = dist.set_column_width(3, 18);
    let _ = dist.set_column_width(4, 12);
    let _ = dist.set_column_width(6, 30);
    let _ = dist.set_column_width(7, 12);
    let _ = dist.set_column_width(9, 12);
    let _ = dist.set_column_width(10, 12);
    let _ = dist.set_column_width(12, 12);
    let _ = dist.set_column_width(13, 12);
    dist.merge_range(0, 0, 0, 13, "Distributions", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(1, 0, "Visible shortlist cap", text_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(1, 1, model.visible_cap as f64, int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(1, 3, "Rows used in distributions", text_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(1, 4, model.all_rows.len() as f64, int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    let approval_counts = approval_label_counts(model);
    dist.write_string_with_format(3, 0, "Approval State", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 1, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (idx, (label, count)) in approval_counts.iter().enumerate() {
        let r = 4 + idx as u32;
        dist.write_string(r, 0, label)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 1, *count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let mut profile_counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &model.all_rows {
        *profile_counts.entry(row.profile_kind.clone()).or_insert(0) += 1;
    }
    dist.write_string_with_format(3, 3, "Profile", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 4, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (idx, (profile, count)) in profile_counts.iter().enumerate() {
        let r = 4 + idx as u32;
        dist.write_string(r, 3, profile)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 4, *count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let reason_counts = top_reason_counts(model, 8);
    dist.write_string_with_format(3, 6, "Top Rejection / Review Reason", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 7, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if reason_counts.is_empty() {
        dist.write_string_with_format(4, 6, "No rejected or review rows", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        for (idx, (reason, count)) in reason_counts.iter().enumerate() {
            let r = 4 + idx as u32;
            dist.write_string(r, 6, reason)
                .map_err(|e: XlsxError| e.to_string())?;
            dist.write_number_with_format(r, 7, *count as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    dist.write_string_with_format(3, 9, "Rank", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 10, "Robust", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 12, "MaxDD", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 13, "Sharpe", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (idx, row) in model.all_rows.iter().enumerate() {
        let r = 4 + idx as u32;
        dist.write_number_with_format(r, 9, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 10, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 12, row.max_dd, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 13, row.sharpe, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !approval_counts.is_empty() {
        let approval_max = 3 + approval_counts.len() as u32;
        let mut approval_chart = Chart::new(ChartType::Column);
        approval_chart
            .add_series()
            .set_categories(("Distributions", 4, 0, approval_max, 0))
            .set_values(("Distributions", 4, 1, approval_max, 1));
        approval_chart.title().set_name("Approval state counts");
        approval_chart.legend().set_hidden();
        dist.insert_chart_with_offset(3, 15, &approval_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !reason_counts.is_empty() {
        let reason_max = 3 + reason_counts.len() as u32;
        let mut reason_chart = Chart::new(ChartType::Bar);
        reason_chart
            .add_series()
            .set_categories(("Distributions", 4, 6, reason_max, 6))
            .set_values(("Distributions", 4, 7, reason_max, 7));
        reason_chart.title().set_name("Top rejection / review reasons");
        reason_chart.legend().set_hidden();
        dist.insert_chart_with_offset(20, 15, &reason_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !profile_counts.is_empty() {
        let profile_max = 3 + profile_counts.len() as u32;
        let mut profile_chart = Chart::new(ChartType::Bar);
        profile_chart
            .add_series()
            .set_categories(("Distributions", 4, 3, profile_max, 3))
            .set_values(("Distributions", 4, 4, profile_max, 4));
        profile_chart.title().set_name("Profile counts");
        profile_chart.legend().set_hidden();
        dist.insert_chart_with_offset(37, 15, &profile_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !model.all_rows.is_empty() {
        let row_min = 4u32;
        let row_max = 3 + model.all_rows.len() as u32;
        let mut robust_chart = Chart::new(ChartType::Line);
        robust_chart
            .add_series()
            .set_categories(("Distributions", row_min, 9, row_max, 9))
            .set_values(("Distributions", row_min, 10, row_max, 10));
        robust_chart.title().set_name("Robust score distribution by rank");
        robust_chart.legend().set_hidden();
        dist.insert_chart_with_offset(54, 15, &robust_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;

        let mut risk_chart = Chart::new(ChartType::Scatter);
        risk_chart
            .add_series()
            .set_categories(("Distributions", row_min, 12, row_max, 12))
            .set_values(("Distributions", row_min, 13, row_max, 13));
        risk_chart.title().set_name("Sharpe vs MaxDD");
        risk_chart.x_axis().set_name("MaxDD");
        risk_chart.y_axis().set_name("Sharpe");
        risk_chart.legend().set_hidden();
        dist.insert_chart_with_offset(71, 15, &risk_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

fn build_formats() -> (
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
    Format,
) {
    let title_fmt = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0F172A))
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);
    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x1E293B))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let label_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF8FAFC))
        .set_border(FormatBorder::Thin);
    let text_fmt = Format::new().set_border(FormatBorder::Thin);
    let mono_fmt = Format::new()
        .set_font_name("Consolas")
        .set_border(FormatBorder::Thin);
    let num_fmt = Format::new()
        .set_num_format("0.000000")
        .set_border(FormatBorder::Thin);
    let pct_fmt = Format::new()
        .set_num_format("0.0000")
        .set_border(FormatBorder::Thin);
    let int_fmt = Format::new().set_num_format("0").set_border(FormatBorder::Thin);
    let approved_fmt = Format::new()
        .set_font_color(Color::RGB(0x166534))
        .set_background_color(Color::RGB(0xDCFCE7))
        .set_border(FormatBorder::Thin);
    let review_fmt = Format::new()
        .set_font_color(Color::RGB(0x92400E))
        .set_background_color(Color::RGB(0xFEF3C7))
        .set_border(FormatBorder::Thin);
    let rejected_fmt = Format::new()
        .set_font_color(Color::RGB(0x991B1B))
        .set_background_color(Color::RGB(0xFEE2E2))
        .set_border(FormatBorder::Thin);
    let compat_fmt = Format::new()
        .set_font_color(Color::RGB(0x1D4ED8))
        .set_background_color(Color::RGB(0xDBEAFE))
        .set_border(FormatBorder::Thin);
    (
        title_fmt,
        header_fmt,
        label_fmt,
        text_fmt,
        mono_fmt,
        num_fmt,
        pct_fmt,
        int_fmt,
        approved_fmt,
        review_fmt,
        rejected_fmt,
        compat_fmt,
    )
}

pub(super) fn build_validation_report_xlsx(
    report: &Value,
    max_rows: Option<usize>,
) -> Result<Vec<u8>, String> {
    let model = build_model_from_report(report, max_rows)?;
    let mut workbook = Workbook::new();
    let (
        title_fmt,
        header_fmt,
        label_fmt,
        text_fmt,
        mono_fmt,
        num_fmt,
        pct_fmt,
        int_fmt,
        approved_fmt,
        review_fmt,
        rejected_fmt,
        compat_fmt,
    ) = build_formats();

    let approved_note = if model.approved_rows.len() > model.visible_cap {
        format!(
            "Showing {} approved rows (of {}).",
            model.visible_cap,
            model.approved_rows.len()
        )
    } else if model.compatibility_mode {
        "Legacy compatibility mode: rows are derived from shortlist compatibility when full verdicts are absent.".to_string()
    } else {
        String::new()
    };
    write_candidate_sheet(
        &mut workbook,
        "Approved Candidates",
        &model
            .approved_rows
            .iter()
            .take(model.visible_cap)
            .cloned()
            .collect::<Vec<_>>(),
        &approved_note,
        &header_fmt,
        &text_fmt,
        &mono_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    let shortlist_note = if model.all_rows.len() > model.visible_cap {
        format!(
            "Visible shortlist capped at {} rows; ordered approved -> hard-gate survivors -> remaining context. Charts and audit use the full set of {} rows.",
            model.visible_cap,
            model.all_rows.len()
        )
    } else if !model.shortlist_rows.is_empty() {
        "Shortlist is ordered approved -> hard-gate survivors -> remaining context.".to_string()
    } else {
        String::new()
    };
    write_candidate_sheet(
        &mut workbook,
        "Shortlist",
        &model.shortlist_rows,
        &shortlist_note,
        &header_fmt,
        &text_fmt,
        &mono_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    build_distributions(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &num_fmt,
        &int_fmt,
    )?;

    let audit_note = if model.compatibility_mode {
        "Audit data includes compatibility-derived approval labels where newer verdict fields are missing."
    } else {
        ""
    };
    write_candidate_sheet(
        &mut workbook,
        "Audit Data",
        &model.all_rows,
        audit_note,
        &header_fmt,
        &text_fmt,
        &mono_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    build_dashboard(
        &mut workbook,
        &model,
        &title_fmt,
        &label_fmt,
        &text_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    workbook
        .save_to_buffer()
        .map_err(|e: XlsxError| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use zip::ZipArchive;

    fn workbook_xml(bytes: &[u8]) -> String {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
        let mut file = zip.by_name("xl/workbook.xml").expect("workbook xml");
        let mut text = String::new();
        file.read_to_string(&mut text).expect("read workbook xml");
        text
    }

    fn has_entry(bytes: &[u8], name: &str) -> bool {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
        let exists = zip.by_name(name).is_ok();
        exists
    }

    fn sheet_xml(bytes: &[u8], idx: usize) -> String {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
        let mut file = zip
            .by_name(&format!("xl/worksheets/sheet{idx}.xml"))
            .expect("sheet xml");
        let mut text = String::new();
        file.read_to_string(&mut text).expect("read sheet xml");
        text
    }

    #[test]
    fn validation_report_workbook_contains_expected_sheets_and_charts() {
        let report = serde_json::json!({
          "version": 1,
          "kind": "vectorbt_report",
          "ts": "2026-03-13T00:00:00Z",
          "run": { "req": { "strategy_kind": "CompositeTrend" } },
          "validation": {
            "res": {
              "data_id": "BTCUSD_1h",
              "backend_used": "CpuOnly",
              "backend_route": "CpuOnly",
              "run_verdict": {
                "passed": true,
                "approved": true,
                "approved_candidate_count": 1,
                "final_recommendation": "single_candidate_supported",
                "first_approved_rank": 1,
                "warning_flags": [],
                "fail_reasons": [],
                "pbo": 0.12
              },
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "ema",
                    "fast_len": 10,
                    "slow_ma_type": "sma",
                    "slow_len": 30
                  },
                  "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.2, "trades": 10, "exposure": 0.5 },
                  "walk_forward": { "sharpe_median": 1.7, "max_dd_worst": 0.25 },
                  "trade_bootstrap": { "prob_pnl_positive": 0.8, "pnl_p50": 0.2, "max_dd_p95": 0.4 },
                  "cross_ticker": { "score": 0.4 },
                  "cross_ticker_pareto": true,
                  "cscv_pbo": { "is_selected_frac": 0.8, "oos_rank_frac_p50": 0.7, "oos_metric_p50": 0.2 },
                  "psr": 0.82,
                  "dsr": 0.78,
                  "track_record_bars": 300,
                  "min_track_record_length_bars": 250,
                  "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
                  "robust_score": 1.23,
                  "robust_pareto": true
                },
                {
                  "params": { "alpha": 2, "beta": 4 },
                  "in_sample": { "pnl": 0.1, "sharpe": 0.3, "max_dd": 0.5, "trades": 5 },
                  "psr": 0.2,
                  "dsr": 0.1,
                  "track_record_bars": 80,
                  "min_track_record_length_bars": 250,
                  "validation_verdict": { "passed": false, "warning_flags": [], "fail_reasons": ["psr_below_min:0.50"] },
                  "robust_score": 0.12,
                  "robust_pareto": false
                }
              ]
            }
          }
        });

        let bytes = build_validation_report_xlsx(&report, Some(25)).expect("xlsx");
        assert_eq!(&bytes[0..2], b"PK");
        assert!(has_entry(&bytes, "xl/charts/chart1.xml"));
        assert!(has_entry(&bytes, "xl/worksheets/sheet5.xml"));
        let workbook = workbook_xml(&bytes);
        assert!(workbook.contains("Approved Candidates"));
        assert!(workbook.contains("Shortlist"));
        assert!(workbook.contains("Distributions"));
        assert!(workbook.contains("Audit Data"));
        assert!(workbook.contains("Dashboard"));
    }

    #[test]
    fn validation_report_empties_approved_sheet_when_run_verdict_rejects_final_approval() {
        let report = serde_json::json!({
          "version": 1,
          "kind": "vectorbt_report",
          "ts": "2026-03-13T00:00:00Z",
          "run": { "req": { "strategy_kind": "Asr" } },
          "validation": {
            "res": {
              "data_id": "run-rejected",
              "backend_used": "CpuOnly",
              "run_verdict": {
                "passed": true,
                "approved": false,
                "candidates_tested": 1,
                "candidates_survived_hard_gates": 1,
                "approved_candidate_count": 1,
                "final_recommendation": "candidates_survived_but_search_not_statistically_superior",
                "warning_flags": ["spa_not_significant"],
                "fail_reasons": [],
                "first_approved_rank": 1,
                "top_candidate_track_record_bars": 320,
                "top_candidate_min_track_record_length_bars": 250,
                "top_candidate_psr": 0.91,
                "top_candidate_dsr": 0.87,
                "pbo": 0.18,
                "max_pbo": 0.30
              },
              "spa": {
                "scope": "search",
                "candidate_count": 1,
                "slice_count": 8,
                "bootstrap_samples": 500,
                "p_value": 0.19,
                "passed": false
              },
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "ema",
                    "slow_ma_type": "sma",
                    "fast_len": 12,
                    "slow_len": 48,
                    "asr": { "angle_deg_min": 10.0, "z_entry": 2.0 }
                  },
                  "in_sample": { "pnl": 1.1, "sharpe": 2.1, "max_dd": 0.2, "trades": 12 },
                  "psr": 0.91,
                  "dsr": 0.87,
                  "track_record_bars": 320,
                  "min_track_record_length_bars": 250,
                  "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
                  "robust_score": 1.4,
                  "robust_pareto": true
                }
              ]
            }
          }
        });

        let bytes = build_validation_report_xlsx(&report, Some(10)).expect("xlsx");
        let approved_sheet = sheet_xml(&bytes, 1);
        assert!(!approved_sheet.contains("r=\"2\""));
        let workbook = workbook_xml(&bytes);
        assert!(workbook.contains("Approved Candidates"));
        assert!(workbook.contains("Dashboard"));
    }


    #[test]
    fn validation_report_supports_legacy_compatibility_mode() {
        let report = serde_json::json!({
          "version": 1,
          "kind": "vectorbt_report",
          "ts": "2026-03-13T00:00:00Z",
          "validation": {
            "res": {
              "data_id": "legacy",
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

        let bytes = build_validation_report_xlsx(&report, Some(10)).expect("xlsx");
        let workbook = workbook_xml(&bytes);
        assert!(workbook.contains("Dashboard"));
        assert!(workbook.contains("Audit Data"));
    }
}
