use super::commands_reports_validation_common::{
    build_model_from_report, run_level_rejection_reason, ExportCandidateRow,
    ExportValidationEvidencePack, ValidationExportModel,
};
use rust_xlsxwriter::{
    utility::quote_sheet_name, Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartSolidFill,
    ChartType, Color, ConditionalFormat3ColorScale, ConditionalFormatCell,
    ConditionalFormatCellRule, ConditionalFormatDataBar, ConditionalFormatType, DocProperties,
    ExcelDateTime, Format, FormatAlign, FormatBorder, Note, Table, TableColumn, TableStyle,
    Workbook, Worksheet, XlsxError,
};
use serde_json::Value;

const XLSX_MAX_ROW_INDEX: usize = 1_048_575;
const XLSX_CELL_CHAR_LIMIT: usize = 32_767;
const SUMMARY_CHART_ROW_CAP: usize = 100;
const TRUNCATION_SUFFIX: &str = " … [truncated for Excel]";

fn excel_safe_text(value: &str) -> String {
    if value.chars().count() <= XLSX_CELL_CHAR_LIMIT {
        return value.to_string();
    }
    let keep = XLSX_CELL_CHAR_LIMIT.saturating_sub(TRUNCATION_SUFFIX.chars().count());
    let mut out: String = value.chars().take(keep).collect();
    out.push_str(TRUNCATION_SUFFIX);
    out
}

fn wrapped_row_height(
    value: &str,
    approximate_chars_per_line: usize,
    minimum_height: f64,
    maximum_height: f64,
) -> f64 {
    let chars_per_line = approximate_chars_per_line.max(1);
    let lines = value
        .chars()
        .count()
        .max(1)
        .saturating_add(chars_per_line - 1)
        / chars_per_line;
    (6.0 + lines as f64 * 15.0)
        .max(minimum_height)
        .min(maximum_height)
}

fn validate_sheet_row_budget(sheet: &str, row_parts: &[usize]) -> Result<(), String> {
    let last_row = row_parts
        .iter()
        .try_fold(0usize, |row, part| row.checked_add(*part))
        .ok_or_else(|| format!("{sheet} worksheet row calculation overflowed"))?;
    if last_row > XLSX_MAX_ROW_INDEX {
        return Err(format!(
            "{sheet} worksheet requires zero-based row index {last_row}; Excel supports at most {XLSX_MAX_ROW_INDEX}"
        ));
    }
    Ok(())
}

fn validate_xlsx_export_limits(model: &ValidationExportModel) -> Result<(), String> {
    // Downside Stress and Distributions place candidate headers on row 3 and
    // data from row 4 onward, so they are the strictest candidate-row layouts.
    validate_sheet_row_budget("candidate analysis", &[3, model.all_rows.len()])?;
    let regime_rows = model
        .all_rows
        .iter()
        .try_fold(0usize, |count, row| {
            count.checked_add(row.regime_bucket_rows.len())
        })
        .ok_or_else(|| "regime audit row count overflowed".to_string())?;
    validate_sheet_row_budget("Regime Audit", &[regime_rows])?;
    let decision_rows = model
        .all_rows
        .iter()
        .try_fold(0usize, |count, row| {
            count.checked_add(candidate_decision_rows(model, row).len())
        })
        .ok_or_else(|| "decision ledger row count overflowed".to_string())?;
    // The Decision Ledger holds a four-row candidate table, a three-row
    // spacer/detail heading block, and then the per-module decision rows.
    validate_sheet_row_budget("Decision Ledger", &[7, model.all_rows.len(), decision_rows])?;
    Ok(())
}

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
const COL_BOOTSTRAP_P05: u16 = 22;
const COL_BOOTSTRAP_DD95: u16 = 23;
const COL_BOOTSTRAP_STAT_BLOCK: u16 = 24;
const COL_BOOTSTRAP_STAT_P05: u16 = 25;
const COL_BOOTSTRAP_STAT_DD95: u16 = 26;
const COL_PERTURB_MEAN: u16 = 27;
const COL_PLATEAU_SCORE: u16 = 28;
const COL_PLATEAU_LABEL: u16 = 29;
const COL_PLATEAU_SOURCE: u16 = 30;
const COL_PLATEAU_NEIGHBORS: u16 = 31;
const COL_PLATEAU_SHARPE: u16 = 32;
const COL_PLATEAU_DD: u16 = 33;
const COL_WF_PNL_EFF: u16 = 34;
const COL_WF_SHARPE_EFF: u16 = 35;
const COL_REGIME_BUCKET: u16 = 36;
const COL_REGIME_SHARPE: u16 = 37;
const COL_REGIME_DD: u16 = 38;
const COL_REGIME_STABLE: u16 = 39;
const COL_REGIME_FLAG: u16 = 40;
const COL_REGIME_SUPPORT: u16 = 41;
const COL_FRICTION_FLAG: u16 = 42;
const COL_RANK_ROBUST: u16 = 43;
const COL_RANK_WF: u16 = 44;
const COL_RANK_CSCV: u16 = 45;
const COL_RANK_CROSS: u16 = 46;
const COL_RANK_DRIFT: u16 = 47;
const COL_WARNINGS: u16 = 48;
const COL_FAIL_REASONS: u16 = 49;
const COL_PARAMS_JSON: u16 = 50;
const COL_MC_MODE: u16 = 51;
const COL_MC_PROB: u16 = 52;
const COL_MC_P05: u16 = 53;
const COL_MC_P50: u16 = 54;
const COL_MC_DD95: u16 = 55;
const COL_MC_STAT_BLOCK: u16 = 56;
const COL_MC_STAT_P05: u16 = 57;
const COL_MC_STAT_DD95: u16 = 58;
const COL_MC_SAMPLES: u16 = 59;
const COL_CANDIDATE_LAST: u16 = COL_MC_SAMPLES;

const CANDIDATE_HEADERS: [&str; 60] = [
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
    "Bootstrap P05",
    "Bootstrap DD95",
    "Stat Block",
    "Stat P05",
    "Stat DD95",
    "Perturb Mean",
    "Plateau Score",
    "Plateau Label",
    "Plateau Src",
    "Plateau N",
    "Plateau Sharpe P50",
    "Plateau DD Worst",
    "WF PnL Eff",
    "WF Sharpe Eff",
    "Regime Worst",
    "Regime Sharpe",
    "Regime MaxDD",
    "Regime Stable",
    "Regime Flag",
    "Regime Support",
    "Friction Flag",
    "Rank Robust",
    "Rank WF",
    "Rank CSCV",
    "Rank Cross",
    "Max Drift",
    "Warnings",
    "Fail Reasons",
    "Params JSON",
    "MC Mode",
    "MC Prob+",
    "MC P05",
    "MC P50",
    "MC DD95",
    "MC Stat Block",
    "MC Stat P05",
    "MC Stat DD95",
    "MC Samples",
];

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

fn display_recommendation_label(label: &str) -> String {
    if label.trim().is_empty() {
        return "No surviving candidates".to_string();
    }
    label.replace('_', " ")
}

fn display_cross_mode_label(label: &str) -> &'static str {
    match label {
        "lightweight" => "Lightweight",
        "preserved_finalists" => "Preserved Finalists",
        "full_stack" => "Full Stack",
        _ => "Unspecified",
    }
}

fn display_cross_semantics_label(label: &str) -> &'static str {
    match label {
        "lightweight" => "lightweight rerun filter",
        "preserved_finalists" => "final post-cross shortlist",
        "full_stack" => "full-stack finalist validation",
        _ => "post-cross candidate summary",
    }
}

fn bootstrap_mode_label(row: &ExportCandidateRow) -> &'static str {
    if row.bootstrap_stationary_mean_block_len.is_some()
        || row.bootstrap_stationary_pnl_p05.is_some()
        || row.bootstrap_stationary_dd_p95.is_some()
    {
        "Compare (IID + Stationary)"
    } else if row.bootstrap_prob_pnl_positive.is_some()
        || row.bootstrap_pnl_p05.is_some()
        || row.bootstrap_dd_p95.is_some()
    {
        "IID only"
    } else {
        "not run"
    }
}

fn display_regime_bucket_label(label: &str) -> String {
    match label {
        "low_vol" => "Low Vol".to_string(),
        "mid_vol" => "Mid Vol".to_string(),
        "high_vol" => "High Vol".to_string(),
        "trend" => "Trend".to_string(),
        "range" => "Range".to_string(),
        _ if label.trim().is_empty() => "Unknown".to_string(),
        _ => label.replace('_', " "),
    }
}

fn display_regime_flag(label: &str) -> &'static str {
    match label {
        "stable" => "Stable",
        "broad" => "Broad",
        "mixed" => "Mixed",
        "narrow" => "Narrow",
        "selective" => "Selective",
        "fragile" => "Fragile",
        _ => "not run",
    }
}

fn display_regime_support(label: &str) -> &'static str {
    match label {
        "broad" => "Broad",
        "trend_ok" => "Trend OK",
        "trend_only" => "Trend Only",
        "range_only" => "Range Only",
        "mixed" => "Mixed",
        "vol_selective" => "Vol Selective",
        "weak" => "Weak",
        _ => "not run",
    }
}

fn summary_mentions_module(summary: &str, module: &str) -> bool {
    fn module_matches_summary_part(part: &str, module: &str) -> bool {
        let normalized = part.trim();
        normalized.eq_ignore_ascii_case(module)
            || normalized
                .split(':')
                .next()
                .map(str::trim)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(module))
    }

    summary
        .split(" + ")
        .any(|part| module_matches_summary_part(part, module))
        || summary
            .split(" | ")
            .any(|part| module_matches_summary_part(part, module))
}

fn disabled_or_omitted_status(model: &ValidationExportModel, module: &str) -> &'static str {
    if summary_mentions_module(&model.actual_disabled_module_summary, module) {
        "disabled"
    } else if summary_mentions_module(&model.actual_skipped_module_summary, module) {
        "skipped"
    } else if summary_mentions_module(&model.actual_cross_omitted_summary, module) {
        "cross omitted"
    } else {
        "not run"
    }
}

fn internal_sheet_link(sheet_name: &str, cell_ref: &str) -> String {
    format!("internal:{}!{}", quote_sheet_name(sheet_name), cell_ref)
}

fn table_columns(headers: &[&str]) -> Vec<TableColumn> {
    headers
        .iter()
        .map(|header| TableColumn::new().set_header(*header))
        .collect()
}

fn add_native_table(
    sheet: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
    name: &str,
    headers: &[&str],
    style: TableStyle,
) -> Result<(), String> {
    if last_row <= first_row {
        return Ok(());
    }
    let columns = table_columns(headers);
    let table = Table::new()
        .set_name(name)
        .set_style(style)
        .set_columns(&columns);
    sheet
        .add_table(first_row, first_col, last_row, last_col, &table)
        .map_err(|e: XlsxError| e.to_string())?;
    Ok(())
}

fn apply_zero_centered_formatting(
    sheet: &mut Worksheet,
    first_row: u32,
    first_col: u16,
    last_row: u32,
    last_col: u16,
) -> Result<(), String> {
    let positive = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::GreaterThan(0))
        .set_format(
            Format::new()
                .set_font_color("166534")
                .set_background_color("DCFCE7"),
        );
    let neutral = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::EqualTo(0))
        .set_format(
            Format::new()
                .set_font_color("92400E")
                .set_background_color("FEF3C7"),
        );
    let negative = ConditionalFormatCell::new()
        .set_rule(ConditionalFormatCellRule::LessThan(0))
        .set_format(
            Format::new()
                .set_font_color("991B1B")
                .set_background_color("FEE2E2"),
        );
    for rule in [&positive, &neutral, &negative] {
        sheet
            .add_conditional_format(first_row, first_col, last_row, last_col, rule)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn probability_data_bar(color: &str) -> ConditionalFormatDataBar {
    ConditionalFormatDataBar::new()
        .set_fill_color(color)
        .set_minimum(ConditionalFormatType::Number, 0)
        .set_maximum(ConditionalFormatType::Number, 1)
}

fn apply_shareable_sheet_layout(sheet: &mut Worksheet, title: &str) {
    sheet
        .set_landscape()
        .set_print_fit_to_pages(1, 0)
        .set_print_center_horizontally(true)
        .set_header(format!("&C{title}"))
        .set_footer("&LVectorGrid&CPage &P of &N&R&D");
}

fn candidate_table_name(sheet_name: &str) -> &'static str {
    match sheet_name {
        "Approved Candidates" => "ApprovedCandidatesTable",
        "Shortlist" => "ShortlistTable",
        "Audit Data" => "AuditDataTable",
        _ => "CandidateRowsTable",
    }
}

fn plateau_summary_label(row: &ExportCandidateRow) -> String {
    if row.plateau_label.trim().is_empty() {
        return "Unavailable".to_string();
    }
    let source_suffix = if row.plateau_source.eq_ignore_ascii_case("top") {
        " | top slice"
    } else {
        ""
    };
    match row.plateau_neighbor_count {
        Some(count) => format!("{} | n={count}{source_suffix}", row.plateau_label),
        None => format!("{}{}", row.plateau_label, source_suffix),
    }
}

fn median_option_f64(values: impl IntoIterator<Item = Option<f64>>) -> Option<f64> {
    let mut xs: Vec<f64> = values
        .into_iter()
        .flatten()
        .filter(|value| value.is_finite())
        .collect();
    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    let mid = xs.len() / 2;
    if xs.len() % 2 == 0 {
        Some((xs[mid - 1] + xs[mid]) / 2.0)
    } else {
        Some(xs[mid])
    }
}

fn approval_label_counts(model: &ValidationExportModel) -> Vec<(String, usize)> {
    use std::collections::BTreeMap;

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in &model.all_rows {
        *counts.entry(row.approval_label.clone()).or_insert(0) += 1;
    }
    if counts.is_empty() {
        if let Some(verdict) = model.run_verdict.as_ref() {
            return vec![(
                format!(
                    "Run Verdict: {}",
                    display_recommendation_label(&verdict.final_recommendation)
                ),
                1,
            )];
        }
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
        if row.final_approved
            || row.approval_reason.trim().is_empty()
            || row.approval_reason == "approved"
        {
            continue;
        }
        *counts.entry(row.approval_reason.clone()).or_insert(0) += 1;
    }
    let mut out: Vec<(String, usize)> = counts.into_iter().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out.truncate(limit);
    if out.is_empty() {
        if let Some(verdict) = model
            .run_verdict
            .as_ref()
            .filter(|verdict| !verdict.approved)
        {
            out.push((run_level_rejection_reason(verdict), 1));
        }
    }
    out
}

fn set_candidate_sheet_layout(sheet: &mut Worksheet) {
    sheet.set_zoom(110);
    sheet.set_screen_gridlines(false);
    let _ = sheet.set_freeze_panes(1, 5);
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
    let _ = sheet.set_column_width(COL_BOOTSTRAP_P05, 12);
    let _ = sheet.set_column_width(COL_BOOTSTRAP_DD95, 12);
    let _ = sheet.set_column_width(COL_BOOTSTRAP_STAT_BLOCK, 11);
    let _ = sheet.set_column_width(COL_BOOTSTRAP_STAT_P05, 12);
    let _ = sheet.set_column_width(COL_BOOTSTRAP_STAT_DD95, 12);
    let _ = sheet.set_column_width(COL_PERTURB_MEAN, 12);
    let _ = sheet.set_column_width(COL_PLATEAU_SCORE, 12);
    let _ = sheet.set_column_width(COL_PLATEAU_LABEL, 16);
    let _ = sheet.set_column_width(COL_PLATEAU_SOURCE, 11);
    let _ = sheet.set_column_width(COL_PLATEAU_NEIGHBORS, 11);
    let _ = sheet.set_column_width(COL_PLATEAU_SHARPE, 13);
    let _ = sheet.set_column_width(COL_PLATEAU_DD, 13);
    let _ = sheet.set_column_width(COL_WF_PNL_EFF, 12);
    let _ = sheet.set_column_width(COL_WF_SHARPE_EFF, 12);
    let _ = sheet.set_column_width(COL_REGIME_BUCKET, 13);
    let _ = sheet.set_column_width(COL_REGIME_SHARPE, 12);
    let _ = sheet.set_column_width(COL_REGIME_DD, 12);
    let _ = sheet.set_column_width(COL_REGIME_STABLE, 12);
    let _ = sheet.set_column_width(COL_REGIME_FLAG, 12);
    let _ = sheet.set_column_width(COL_REGIME_SUPPORT, 14);
    let _ = sheet.set_column_width(COL_RANK_ROBUST, 11);
    let _ = sheet.set_column_width(COL_RANK_WF, 9);
    let _ = sheet.set_column_width(COL_RANK_CSCV, 11);
    let _ = sheet.set_column_width(COL_RANK_CROSS, 11);
    let _ = sheet.set_column_width(COL_RANK_DRIFT, 11);
    let _ = sheet.set_column_width(COL_WARNINGS, 28);
    let _ = sheet.set_column_width(COL_FAIL_REASONS, 30);
    let _ = sheet.set_column_width(COL_PARAMS_JSON, 44);
    let _ = sheet.set_column_width(COL_MC_MODE, 18);
    let _ = sheet.set_column_width(COL_MC_PROB, 12);
    let _ = sheet.set_column_width(COL_MC_P05, 12);
    let _ = sheet.set_column_width(COL_MC_P50, 12);
    let _ = sheet.set_column_width(COL_MC_DD95, 12);
    let _ = sheet.set_column_width(COL_MC_STAT_BLOCK, 13);
    let _ = sheet.set_column_width(COL_MC_STAT_P05, 13);
    let _ = sheet.set_column_width(COL_MC_STAT_DD95, 13);
    let _ = sheet.set_column_width(COL_MC_SAMPLES, 12);
}

fn write_candidate_headers(sheet: &mut Worksheet, header_fmt: &Format) -> Result<(), String> {
    for (col, header) in CANDIDATE_HEADERS.iter().enumerate() {
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
            .write_string_with_format(r, COL_PROFILE, excel_safe_text(&row.profile_kind), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_PARAMS_SUMMARY,
                excel_safe_text(&row.params_summary),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_ROBUST, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, COL_PSR, row.psr, pct_fmt)?;
        write_optional_number(sheet, r, COL_DSR, row.dsr, pct_fmt)?;
        write_optional_usize(sheet, r, COL_TRACK_BARS, row.track_record_bars, int_fmt)?;
        write_optional_usize(
            sheet,
            r,
            COL_MINTRL,
            row.min_track_record_length_bars,
            int_fmt,
        )?;
        sheet
            .write_number_with_format(r, COL_PNL, row.pnl, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_SHARPE, row.sharpe, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_MAX_DD, row.max_dd, pct_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, COL_TRADES, row.trades, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, COL_WF_SHARPE, row.wf_sharpe_median, num_fmt)?;
        write_optional_number(sheet, r, COL_WF_DD, row.wf_dd_worst, pct_fmt)?;
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
        write_optional_number(sheet, r, COL_BOOTSTRAP_P05, row.bootstrap_pnl_p05, num_fmt)?;
        write_optional_number(sheet, r, COL_BOOTSTRAP_DD95, row.bootstrap_dd_p95, pct_fmt)?;
        write_optional_usize(
            sheet,
            r,
            COL_BOOTSTRAP_STAT_BLOCK,
            row.bootstrap_stationary_mean_block_len,
            int_fmt,
        )?;
        write_optional_number(
            sheet,
            r,
            COL_BOOTSTRAP_STAT_P05,
            row.bootstrap_stationary_pnl_p05,
            num_fmt,
        )?;
        write_optional_number(
            sheet,
            r,
            COL_BOOTSTRAP_STAT_DD95,
            row.bootstrap_stationary_dd_p95,
            pct_fmt,
        )?;
        write_optional_number(sheet, r, COL_PERTURB_MEAN, row.perturb_pnl_mean, num_fmt)?;
        write_optional_number(sheet, r, COL_PLATEAU_SCORE, row.plateau_score, num_fmt)?;
        sheet
            .write_string_with_format(
                r,
                COL_PLATEAU_LABEL,
                excel_safe_text(if row.plateau_label.is_empty() {
                    "unavailable"
                } else {
                    row.plateau_label.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_PLATEAU_SOURCE,
                excel_safe_text(if row.plateau_source.is_empty() {
                    "-"
                } else {
                    row.plateau_source.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(
            sheet,
            r,
            COL_PLATEAU_NEIGHBORS,
            row.plateau_neighbor_count,
            int_fmt,
        )?;
        write_optional_number(
            sheet,
            r,
            COL_PLATEAU_SHARPE,
            row.plateau_sharpe_median,
            num_fmt,
        )?;
        write_optional_number(sheet, r, COL_PLATEAU_DD, row.plateau_max_dd_worst, pct_fmt)?;
        write_optional_number(sheet, r, COL_WF_PNL_EFF, row.wf_pnl_efficiency, num_fmt)?;
        write_optional_number(
            sheet,
            r,
            COL_WF_SHARPE_EFF,
            row.wf_sharpe_efficiency,
            num_fmt,
        )?;
        sheet
            .write_string_with_format(
                r,
                COL_REGIME_BUCKET,
                excel_safe_text(if row.regime_worst_bucket.is_empty() {
                    "not run"
                } else {
                    row.regime_worst_bucket.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(
            sheet,
            r,
            COL_REGIME_SHARPE,
            row.regime_worst_sharpe,
            num_fmt,
        )?;
        write_optional_number(sheet, r, COL_REGIME_DD, row.regime_worst_max_dd, pct_fmt)?;
        write_optional_usize(
            sheet,
            r,
            COL_REGIME_STABLE,
            row.regime_stable_bucket_count,
            int_fmt,
        )?;
        sheet
            .write_string_with_format(
                r,
                COL_REGIME_FLAG,
                display_regime_flag(&row.regime_fragility_flag),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_REGIME_SUPPORT,
                display_regime_support(&row.regime_support_label),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_FRICTION_FLAG,
                excel_safe_text(if row.friction_flag.is_empty() {
                    "not run"
                } else {
                    row.friction_flag.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(sheet, r, COL_RANK_ROBUST, row.robust_view_rank, int_fmt)?;
        write_optional_usize(sheet, r, COL_RANK_WF, row.wf_view_rank, int_fmt)?;
        write_optional_usize(sheet, r, COL_RANK_CSCV, row.cscv_view_rank, int_fmt)?;
        write_optional_usize(sheet, r, COL_RANK_CROSS, row.cross_view_rank, int_fmt)?;
        write_optional_usize(sheet, r, COL_RANK_DRIFT, row.max_view_rank_drift, int_fmt)?;
        sheet
            .write_string_with_format(
                r,
                COL_WARNINGS,
                excel_safe_text(&row.warning_flags),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_FAIL_REASONS,
                excel_safe_text(&row.fail_reasons),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_PARAMS_JSON,
                excel_safe_text(&row.params_json),
                mono_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                COL_MC_MODE,
                excel_safe_text(&row.monte_carlo_mode),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(
            sheet,
            r,
            COL_MC_PROB,
            row.monte_carlo_prob_pnl_positive,
            pct_fmt,
        )?;
        write_optional_number(sheet, r, COL_MC_P05, row.monte_carlo_pnl_p05, num_fmt)?;
        write_optional_number(sheet, r, COL_MC_P50, row.monte_carlo_pnl_p50, num_fmt)?;
        write_optional_number(sheet, r, COL_MC_DD95, row.monte_carlo_max_dd_p95, pct_fmt)?;
        write_optional_usize(
            sheet,
            r,
            COL_MC_STAT_BLOCK,
            row.monte_carlo_stationary_mean_block_len,
            int_fmt,
        )?;
        write_optional_number(
            sheet,
            r,
            COL_MC_STAT_P05,
            row.monte_carlo_stationary_pnl_p05,
            num_fmt,
        )?;
        write_optional_number(
            sheet,
            r,
            COL_MC_STAT_DD95,
            row.monte_carlo_stationary_max_dd_p95,
            pct_fmt,
        )?;
        write_optional_usize(
            sheet,
            r,
            COL_MC_SAMPLES,
            row.monte_carlo_sample_count,
            int_fmt,
        )?;
    }
    Ok(())
}

fn apply_candidate_sheet_analysis_formatting(
    sheet: &mut Worksheet,
    row_count: usize,
    table_name: &str,
) -> Result<(), String> {
    if row_count == 0 {
        return Ok(());
    }

    let row_min = 1u32;
    let row_max = row_count as u32;
    add_native_table(
        sheet,
        0,
        0,
        row_max,
        COL_CANDIDATE_LAST,
        table_name,
        &CANDIDATE_HEADERS,
        TableStyle::Medium2,
    )?;

    let score_bar = ConditionalFormatDataBar::new().set_fill_color("63BE7B");
    let support_bar = ConditionalFormatDataBar::new().set_fill_color("5A8AC6");
    let good_scale = ConditionalFormat3ColorScale::new()
        .set_minimum_color("F8696B")
        .set_midpoint_color("FFEB84")
        .set_maximum_color("63BE7B");
    let bad_scale = ConditionalFormat3ColorScale::new()
        .set_minimum_color("63BE7B")
        .set_midpoint_color("FFEB84")
        .set_maximum_color("F8696B");

    for col in [
        COL_ROBUST,
        COL_PSR,
        COL_DSR,
        COL_CSCV_RANK,
        COL_CSCV_SEL,
        COL_CROSS_SCORE,
        COL_BOOTSTRAP_PROB,
        COL_PLATEAU_SCORE,
        COL_WF_PNL_EFF,
        COL_WF_SHARPE_EFF,
    ] {
        sheet
            .add_conditional_format(row_min, col, row_max, col, &score_bar)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    for col in [
        COL_PNL,
        COL_SHARPE,
        COL_WF_SHARPE,
        COL_PERTURB_MEAN,
        COL_PLATEAU_SHARPE,
        COL_REGIME_SHARPE,
    ] {
        sheet
            .add_conditional_format(row_min, col, row_max, col, &good_scale)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    for col in [
        COL_MAX_DD,
        COL_WF_DD,
        COL_BOOTSTRAP_DD95,
        COL_BOOTSTRAP_STAT_DD95,
        COL_PLATEAU_DD,
        COL_REGIME_DD,
        COL_RANK_DRIFT,
    ] {
        sheet
            .add_conditional_format(row_min, col, row_max, col, &bad_scale)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    for col in [COL_TRACK_BARS, COL_REGIME_STABLE] {
        sheet
            .add_conditional_format(row_min, col, row_max, col, &support_bar)
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
    let tab_color = match name {
        "Approved Candidates" => Color::RGB(0x22C55E),
        "Shortlist" => Color::RGB(0x3B82F6),
        "Audit Data" => Color::RGB(0x64748B),
        _ => Color::RGB(0x94A3B8),
    };
    sheet.set_tab_color(tab_color);
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
    apply_candidate_sheet_analysis_formatting(sheet, rows.len(), candidate_table_name(name))?;
    if !title_note.is_empty() {
        let note = Note::new(title_note).set_author("VectorGrid");
        sheet
            .insert_note(0, 0, &note)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn write_regime_audit_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Regime Audit")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0x14B8A6));
    sheet.set_screen_gridlines(false);
    apply_shareable_sheet_layout(sheet, "VectorGrid Regime Audit");
    let _ = sheet.set_freeze_panes(1, 3);
    let wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);
    for (col, width) in [
        (0u16, 8.0),
        (1u16, 14.0),
        (2u16, 52.0),
        (3u16, 12.0),
        (4u16, 10.0),
        (5u16, 10.0),
        (6u16, 12.0),
        (7u16, 12.0),
        (8u16, 12.0),
        (9u16, 10.0),
        (10u16, 10.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }
    for (col, header) in [
        "Rank", "Approval", "Params", "Slice", "Bars", "Active", "PnL", "Sharpe", "MaxDD",
        "Trades", "Stable",
    ]
    .iter()
    .enumerate()
    {
        sheet
            .write_string_with_format(0, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let mut row_idx = 1u32;
    for row in &model.all_rows {
        for bucket in &row.regime_bucket_rows {
            let _ = sheet.set_row_height(row_idx, 30.0);
            sheet
                .write_number_with_format(row_idx, 0, row.rank as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string_with_format(
                    row_idx,
                    1,
                    display_approval_label(&row.approval_label),
                    text_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string_with_format(
                    row_idx,
                    2,
                    excel_safe_text(&row.params_summary),
                    &wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string_with_format(
                    row_idx,
                    3,
                    excel_safe_text(&display_regime_bucket_label(&bucket.label)),
                    text_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            write_optional_usize(sheet, row_idx, 4, bucket.bars, int_fmt)?;
            write_optional_number(sheet, row_idx, 5, bucket.active_frac, pct_fmt)?;
            write_optional_number(sheet, row_idx, 6, bucket.pnl, num_fmt)?;
            write_optional_number(sheet, row_idx, 7, bucket.sharpe, num_fmt)?;
            write_optional_number(sheet, row_idx, 8, bucket.max_dd, pct_fmt)?;
            write_optional_usize(sheet, row_idx, 9, bucket.trades, int_fmt)?;
            sheet
                .write_string_with_format(
                    row_idx,
                    10,
                    if bucket.stable == Some(true) {
                        "yes"
                    } else if bucket.stable == Some(false) {
                        "no"
                    } else {
                        "n/a"
                    },
                    text_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            row_idx += 1;
        }
    }
    if row_idx > 1 {
        let row_max = row_idx - 1;
        add_native_table(
            sheet,
            0,
            0,
            row_max,
            10,
            "RegimeAuditTable",
            &[
                "Rank", "Approval", "Params", "Slice", "Bars", "Active", "PnL", "Sharpe", "MaxDD",
                "Trades", "Stable",
            ],
            TableStyle::Medium4,
        )?;
        let support_bar = ConditionalFormatDataBar::new().set_fill_color("5A8AC6");
        let good_scale = ConditionalFormat3ColorScale::new()
            .set_minimum_color("F8696B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("63BE7B");
        let bad_scale = ConditionalFormat3ColorScale::new()
            .set_minimum_color("63BE7B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("F8696B");
        sheet
            .add_conditional_format(1, 5, row_max, 5, &support_bar)
            .map_err(|e: XlsxError| e.to_string())?;
        for col in [6u16, 7u16] {
            sheet
                .add_conditional_format(1, col, row_max, col, &good_scale)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        sheet
            .add_conditional_format(1, 8, row_max, 8, &bad_scale)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    if row_idx == 1 {
        sheet
            .write_string_with_format(
                1,
                0,
                "No regime slice robustness rows present in this artifact.",
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn format_optional_number(value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite())
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "not run".to_string())
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "not run".to_string())
}

fn count_axis_major_unit(max_count: usize) -> f64 {
    if max_count <= 5 {
        1.0
    } else {
        (max_count as f64 / 5.0).ceil()
    }
}

fn capped_chart_title(base: &str, total_rows: usize) -> String {
    if total_rows > SUMMARY_CHART_ROW_CAP {
        format!("{base} (top {SUMMARY_CHART_ROW_CAP})")
    } else {
        base.to_string()
    }
}

fn min_threshold_status(value: Option<f64>, threshold: Option<f64>) -> (&'static str, String) {
    match (value.filter(|v| v.is_finite()), threshold) {
        (Some(value), Some(threshold)) if value >= threshold => {
            ("pass", format!(">= {threshold:.4}"))
        }
        (Some(_), Some(threshold)) => ("fail", format!(">= {threshold:.4}")),
        (Some(_), None) => ("observed", "threshold off".to_string()),
        (None, Some(threshold)) => ("not run", format!(">= {threshold:.4}")),
        (None, None) => ("not run", "threshold off".to_string()),
    }
}

fn max_threshold_status(value: Option<f64>, threshold: Option<f64>) -> (&'static str, String) {
    match (value.filter(|v| v.is_finite()), threshold) {
        (Some(value), Some(threshold)) if value <= threshold => {
            ("pass", format!("<= {threshold:.4}"))
        }
        (Some(_), Some(threshold)) => ("fail", format!("<= {threshold:.4}")),
        (Some(_), None) => ("observed", "threshold off".to_string()),
        (None, Some(threshold)) => ("not run", format!("<= {threshold:.4}")),
        (None, None) => ("not run", "threshold off".to_string()),
    }
}

fn status_format<'a>(
    status: &str,
    text_fmt: &'a Format,
    approved_fmt: &'a Format,
    review_fmt: &'a Format,
    rejected_fmt: &'a Format,
    compat_fmt: &'a Format,
) -> &'a Format {
    match status {
        "pass" | "completed" => approved_fmt,
        "watch" | "observed" => review_fmt,
        "fail" => rejected_fmt,
        "compat" | "disabled" | "skipped" | "cross omitted" | "not run" => compat_fmt,
        _ => text_fmt,
    }
}

fn friction_status(label: &str) -> &'static str {
    let normalized = label.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "not run" {
        "not run"
    } else if normalized.contains("fail") || normalized.contains("fragile") {
        "fail"
    } else if normalized.contains("pass") || normalized.contains("stable") {
        "pass"
    } else {
        "watch"
    }
}

fn regime_status(label: &str) -> &'static str {
    match label {
        "stable" | "broad" => "pass",
        "selective" | "mixed" | "narrow" => "watch",
        "fragile" => "fail",
        _ => "not run",
    }
}

fn selected_candidate(model: &ValidationExportModel) -> Option<&ExportCandidateRow> {
    model
        .approved_rows
        .first()
        .or_else(|| model.shortlist_rows.first())
        .or_else(|| model.all_rows.first())
}

fn any_candidate_has(
    model: &ValidationExportModel,
    predicate: impl Fn(&ExportCandidateRow) -> bool,
) -> bool {
    model.all_rows.iter().any(predicate)
}

fn module_feature_status(model: &ValidationExportModel, module: &str, has_result: bool) -> String {
    if has_result {
        "completed".to_string()
    } else {
        disabled_or_omitted_status(model, module).to_string()
    }
}

struct ValidationFeatureRow {
    feature: String,
    scope: String,
    status: String,
    evidence: String,
    key_metrics: String,
    gate_or_policy: String,
    inspect: String,
    notes: String,
}

fn validation_feature_rows(model: &ValidationExportModel) -> Vec<ValidationFeatureRow> {
    let top = selected_candidate(model);
    let run_verdict = model.run_verdict.as_ref();
    let mut rows = Vec::new();

    rows.push(ValidationFeatureRow {
        feature: "Final run verdict".to_string(),
        scope: "Run".to_string(),
        status: run_verdict
            .map(|verdict| {
                if verdict.approved {
                    "pass"
                } else if verdict.passed {
                    "watch"
                } else {
                    "fail"
                }
                .to_string()
            })
            .unwrap_or_else(|| "compat".to_string()),
        evidence: run_verdict
            .map(|verdict| {
                format!(
                    "approved={} passed={} recommendation={}",
                    verdict.approved,
                    verdict.passed,
                    display_recommendation_label(&verdict.final_recommendation)
                )
            })
            .unwrap_or_else(|| "legacy artifact without explicit run verdict".to_string()),
        key_metrics: run_verdict
            .map(|verdict| {
                format!(
                    "approved candidates={} first approved rank={}",
                    verdict.approved_candidate_count,
                    format_optional_usize(verdict.first_approved_rank)
                )
            })
            .unwrap_or_else(|| format!("approved candidates={}", model.approved_rows.len())),
        gate_or_policy: "Run-level approval contract".to_string(),
        inspect: "Dashboard; Approved Candidates; Rejection Diagnostics".to_string(),
        notes: "Separates a technically passed validation run from a final approved result."
            .to_string(),
    });

    let candidates_tested = run_verdict
        .map(|verdict| verdict.candidates_tested)
        .unwrap_or_else(|| model.all_rows.len());
    let survived_hard_gates = run_verdict
        .map(|verdict| verdict.candidates_survived_hard_gates)
        .unwrap_or_else(|| {
            model
                .all_rows
                .iter()
                .filter(|row| row.hard_gate_passed == Some(true))
                .count()
        });
    let hard_gate_recorded = any_candidate_has(model, |row| row.hard_gate_passed.is_some());
    rows.push(ValidationFeatureRow {
        feature: "Candidate hard gates".to_string(),
        scope: "Candidate".to_string(),
        status: if hard_gate_recorded || run_verdict.is_some() {
            if survived_hard_gates > 0 {
                "completed"
            } else {
                "fail"
            }
        } else {
            "compat"
        }
        .to_string(),
        evidence: format!("{survived_hard_gates}/{candidates_tested} candidates survived"),
        key_metrics: top
            .map(|row| {
                format!(
                    "top hard gate={}",
                    match row.hard_gate_passed {
                        Some(true) => "pass",
                        Some(false) => "fail",
                        None => "not recorded",
                    }
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        gate_or_policy: "Candidate validation_verdict and run hard-gate rebuild".to_string(),
        inspect: "Approved Candidates; Shortlist; Audit Data".to_string(),
        notes: "Primary candidate-level pass/fail surface before explanatory robustness views."
            .to_string(),
    });

    for (feature, value, threshold, metric_name) in [
        (
            "Probabilistic Sharpe Ratio",
            top.and_then(|row| row.psr),
            model.min_psr,
            "PSR",
        ),
        (
            "Deflated Sharpe Ratio",
            top.and_then(|row| row.dsr),
            model.min_dsr,
            "DSR",
        ),
    ] {
        let (status, reference) = min_threshold_status(value, threshold);
        rows.push(ValidationFeatureRow {
            feature: feature.to_string(),
            scope: "Candidate".to_string(),
            status: status.to_string(),
            evidence: format!("{metric_name}={}", format_optional_number(value)),
            key_metrics: format!("top candidate {metric_name}"),
            gate_or_policy: reference,
            inspect: "Dashboard; Robustness Matrix; Rejection Diagnostics".to_string(),
            notes: "Probability metric used by hard-gate logic when configured.".to_string(),
        });
    }

    let track_status = top
        .map(
            |row| match (row.track_record_bars, row.min_track_record_length_bars) {
                (Some(observed), Some(minimum)) if observed >= minimum => "pass",
                (Some(_), Some(_)) => "fail",
                (Some(_), None) => "observed",
                _ => "not run",
            },
        )
        .unwrap_or("not run");
    rows.push(ValidationFeatureRow {
        feature: "Track record length".to_string(),
        scope: "Candidate".to_string(),
        status: track_status.to_string(),
        evidence: top
            .map(|row| {
                format!(
                    "{}/{} bars",
                    format_optional_usize(row.track_record_bars),
                    format_optional_usize(row.min_track_record_length_bars)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: "observed bars vs minimum required bars".to_string(),
        gate_or_policy: "observed >= minimum".to_string(),
        inspect: "Dashboard; Rejection Diagnostics".to_string(),
        notes: "Guards probability metrics from too-short realized trade histories.".to_string(),
    });

    let has_walk_forward = any_candidate_has(model, |row| {
        row.wf_sharpe_median.is_some()
            || row.wf_dd_worst.is_some()
            || row.wf_pnl_efficiency.is_some()
            || row.wf_sharpe_efficiency.is_some()
    });
    rows.push(ValidationFeatureRow {
        feature: "Walk-forward candidate validation".to_string(),
        scope: "Candidate".to_string(),
        status: module_feature_status(model, "walk_forward", has_walk_forward),
        evidence: top
            .map(|row| {
                format!(
                    "Sharpe p50={} DD worst={}",
                    format_optional_number(row.wf_sharpe_median),
                    format_optional_number(row.wf_dd_worst)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: top
            .map(|row| {
                format!(
                    "WFE pnl={} sharpe={}",
                    format_optional_number(row.wf_pnl_efficiency),
                    format_optional_number(row.wf_sharpe_efficiency)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: model.wfe_policy_summary.clone(),
        inspect: "Dashboard; Robustness Matrix; Distributions".to_string(),
        notes: "Shows validation-window survival and efficiency against in-sample behavior."
            .to_string(),
    });

    rows.push(ValidationFeatureRow {
        feature: "Reoptimized walk-forward".to_string(),
        scope: "Run".to_string(),
        status: module_feature_status(
            model,
            "reopt_walk_forward",
            model.reopt_walk_forward.is_some(),
        ),
        evidence: model
            .reopt_walk_forward
            .as_ref()
            .map(|summary| {
                format!(
                    "windows={} winners={} changes={}",
                    summary.windows, summary.unique_winners, summary.winner_changes
                )
            })
            .unwrap_or_else(|| disabled_or_omitted_status(model, "reopt_walk_forward").to_string()),
        key_metrics: model
            .reopt_walk_forward
            .as_ref()
            .map(|summary| {
                format!(
                    "fwd pnl+={} fwd sharpe={} WFE sharpe p50={}",
                    format_optional_number(summary.forward_positive_pnl_frac),
                    format_optional_number(summary.forward_sharpe_median),
                    format_optional_number(summary.wfe_sharpe_median)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: "Run-level stability diagnostic".to_string(),
        inspect: "Dashboard".to_string(),
        notes: "Checks whether repeated optimization keeps finding similar forward winners."
            .to_string(),
    });

    let cscv_has_result = model
        .cscv_pbo
        .as_ref()
        .is_some_and(|cscv| cscv.skipped_reason.is_empty());
    let cscv_status = if let Some(cscv) = model.cscv_pbo.as_ref() {
        if !cscv.skipped_reason.is_empty() {
            "skipped".to_string()
        } else {
            let threshold = model.max_pbo.or(cscv.threshold);
            max_threshold_status(cscv.pbo, threshold).0.to_string()
        }
    } else {
        disabled_or_omitted_status(model, "cscv_pbo").to_string()
    };
    rows.push(ValidationFeatureRow {
        feature: "CSCV / PBO".to_string(),
        scope: "Run + Candidate".to_string(),
        status: cscv_status,
        evidence: model
            .cscv_pbo
            .as_ref()
            .map(|summary| {
                if summary.skipped_reason.is_empty() {
                    format!(
                        "slices={} combinations={} pbo={}",
                        summary.slices,
                        summary.combinations,
                        format_optional_number(summary.pbo)
                    )
                } else {
                    format!("skipped: {}", summary.skipped_reason)
                }
            })
            .unwrap_or_else(|| disabled_or_omitted_status(model, "cscv_pbo").to_string()),
        key_metrics: top
            .map(|row| {
                format!(
                    "rank frac={} metric={} selected={}",
                    format_optional_number(row.cscv_oos_rank_frac_p50),
                    format_optional_number(row.cscv_oos_metric_p50),
                    format_optional_number(row.cscv_selected_frac)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: format!(
            "max PBO={} min rank={} min metric={}",
            format_optional_number(model.max_pbo),
            format_optional_number(model.min_cscv_oos_rank_frac),
            format_optional_number(model.min_cscv_oos_metric)
        ),
        inspect: "Dashboard; Robustness Matrix; Rejection Diagnostics".to_string(),
        notes: if cscv_has_result {
            "Run-level overfitting probability plus candidate OOS rank/metric statistics."
        } else {
            "Unavailable or skipped in this artifact."
        }
        .to_string(),
    });

    rows.push(ValidationFeatureRow {
        feature: "SPA statistical superiority".to_string(),
        scope: "Run".to_string(),
        status: model
            .spa
            .as_ref()
            .map(|spa| {
                if !spa.skipped_reason.is_empty() {
                    "skipped"
                } else if spa.passed == Some(true) {
                    "pass"
                } else if spa.passed == Some(false) {
                    "fail"
                } else {
                    "observed"
                }
                .to_string()
            })
            .unwrap_or_else(|| disabled_or_omitted_status(model, "spa").to_string()),
        evidence: model
            .spa
            .as_ref()
            .map(|spa| {
                if spa.skipped_reason.is_empty() {
                    format!(
                        "p={} candidates={} slices={} boot={}",
                        format_optional_number(spa.p_value),
                        spa.candidate_count,
                        spa.slice_count,
                        spa.bootstrap_samples
                    )
                } else {
                    format!("skipped: {}", spa.skipped_reason)
                }
            })
            .unwrap_or_else(|| disabled_or_omitted_status(model, "spa").to_string()),
        key_metrics: "p-value against configured benchmark".to_string(),
        gate_or_policy: "Lower p-value is stronger; pass flag comes from validator.".to_string(),
        inspect: "Dashboard; Rejection Diagnostics".to_string(),
        notes: "Run-level check that surviving candidates outperform the benchmark beyond luck."
            .to_string(),
    });

    rows.push(ValidationFeatureRow {
        feature: "Cross-ticker / cross-dataset".to_string(),
        scope: "Run + Candidate".to_string(),
        status: module_feature_status(model, "cross_ticker", model.cross_ticker.is_some()),
        evidence: model
            .cross_ticker
            .as_ref()
            .map(|cross| {
                format!(
                    "datasets={} mode={} shortlist={}",
                    cross.dataset_count,
                    display_cross_mode_label(&cross.mode),
                    cross.shortlist_count
                )
            })
            .unwrap_or_else(|| disabled_or_omitted_status(model, "cross_ticker").to_string()),
        key_metrics: top
            .map(|row| {
                format!(
                    "score={} pareto={}",
                    format_optional_number(row.cross_score),
                    if row.cross_pareto { "yes" } else { "no" }
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: model.expected_cross_summary.clone(),
        inspect: "Dashboard; Robustness Matrix; Candidate sheets".to_string(),
        notes: "Shows whether finalists survive the configured comparison datasets.".to_string(),
    });

    let has_bootstrap = any_candidate_has(model, |row| {
        row.bootstrap_prob_pnl_positive.is_some()
            || row.bootstrap_pnl_p05.is_some()
            || row.bootstrap_dd_p95.is_some()
            || row.bootstrap_stationary_pnl_p05.is_some()
            || row.bootstrap_stationary_dd_p95.is_some()
    });
    rows.push(ValidationFeatureRow {
        feature: "Closed-trade bootstrap".to_string(),
        scope: "Candidate".to_string(),
        status: module_feature_status(model, "trade_bootstrap", has_bootstrap),
        evidence: top
            .map(|row| {
                format!(
                    "prob+={} iid p05={} stat p05={}",
                    format_optional_number(row.bootstrap_prob_pnl_positive),
                    format_optional_number(row.bootstrap_pnl_p05),
                    format_optional_number(row.bootstrap_stationary_pnl_p05)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: top
            .map(|row| bootstrap_mode_label(row).to_string())
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: model.bootstrap_mode_summary.clone(),
        inspect: "Dashboard; Robustness Matrix; Candidate sheets".to_string(),
        notes: "Resamples closed trades to expose downside and positive-PnL probability."
            .to_string(),
    });

    let has_perturb = any_candidate_has(model, |row| {
        row.perturb_pnl_mean.is_some()
            || row.perturb_pnl_worst.is_some()
            || row.perturb_positive_frac.is_some()
    });
    rows.push(ValidationFeatureRow {
        feature: "Parameter perturbation".to_string(),
        scope: "Candidate".to_string(),
        status: module_feature_status(model, "perturb", has_perturb),
        evidence: top
            .map(|row| {
                format!(
                    "mean={} worst={} positive={}",
                    format_optional_number(row.perturb_pnl_mean),
                    format_optional_number(row.perturb_pnl_worst),
                    format_optional_number(row.perturb_positive_frac)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: "PnL under randomized parameter deviations".to_string(),
        gate_or_policy: "Auxiliary robustness diagnostic".to_string(),
        inspect: "Candidate sheets; Robustness Matrix".to_string(),
        notes: "Highlights parameter brittleness around finalist settings.".to_string(),
    });

    let has_plateau = any_candidate_has(model, |row| row.plateau_score.is_some());
    rows.push(ValidationFeatureRow {
        feature: "Local parameter plateau".to_string(),
        scope: "Candidate".to_string(),
        status: if has_plateau {
            "observed".to_string()
        } else {
            "not run".to_string()
        },
        evidence: top
            .map(|row| {
                format!(
                    "score={} label={}",
                    format_optional_number(row.plateau_score),
                    plateau_summary_label(row)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: top
            .map(|row| {
                format!(
                    "neighbors={} sharpe p50={} DD worst={}",
                    format_optional_usize(row.plateau_neighbor_count),
                    format_optional_number(row.plateau_sharpe_median),
                    format_optional_number(row.plateau_max_dd_worst)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: "Derived report-side surface diagnostic".to_string(),
        inspect: "Dashboard; Candidate sheets; Robustness Matrix".to_string(),
        notes: "Uses available optimization surface points to show whether a candidate sits on a plateau."
            .to_string(),
    });

    let has_regime = any_candidate_has(model, |row| {
        !row.regime_bucket_rows.is_empty()
            || row.regime_stable_bucket_frac.is_some()
            || row.regime_worst_sharpe.is_some()
    });
    rows.push(ValidationFeatureRow {
        feature: "Regime slice robustness".to_string(),
        scope: "Candidate".to_string(),
        status: top
            .filter(|_| has_regime)
            .map(|row| regime_status(&row.regime_fragility_flag).to_string())
            .unwrap_or_else(|| "not run".to_string()),
        evidence: top
            .map(|row| {
                format!(
                    "flag={} support={} stable={}",
                    display_regime_flag(&row.regime_fragility_flag),
                    display_regime_support(&row.regime_support_label),
                    format_optional_number(row.regime_stable_bucket_frac)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: top
            .map(|row| {
                format!(
                    "worst={} sharpe={} maxdd={}",
                    if row.regime_worst_bucket.is_empty() {
                        "not run"
                    } else {
                        row.regime_worst_bucket.as_str()
                    },
                    format_optional_number(row.regime_worst_sharpe),
                    format_optional_number(row.regime_worst_max_dd)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: "Descriptive regime stability classification".to_string(),
        inspect: "Regime Audit; Dashboard; Robustness Matrix".to_string(),
        notes: "Breaks robustness into low/mid/high volatility plus trend/range slices."
            .to_string(),
    });

    let has_friction = any_candidate_has(model, |row| {
        row.friction_pass_frac.is_some()
            || row.friction_worst_sharpe.is_some()
            || row.friction_worst_max_dd.is_some()
            || row.friction_max_rank_drift.is_some()
    });
    rows.push(ValidationFeatureRow {
        feature: "Friction stress".to_string(),
        scope: "Candidate".to_string(),
        status: top
            .filter(|_| has_friction)
            .map(|row| friction_status(&row.friction_flag).to_string())
            .unwrap_or_else(|| "not run".to_string()),
        evidence: top
            .map(|row| {
                format!(
                    "pass frac={} drift={}",
                    format_optional_number(row.friction_pass_frac),
                    format_optional_usize(row.friction_max_rank_drift)
                )
            })
            .unwrap_or_else(|| "no candidate rows".to_string()),
        key_metrics: top
            .map(|row| {
                format!(
                    "worst={} sharpe={} maxdd={}",
                    if row.friction_worst_scenario.is_empty() {
                        "not run"
                    } else {
                        row.friction_worst_scenario.as_str()
                    },
                    format_optional_number(row.friction_worst_sharpe),
                    format_optional_number(row.friction_worst_max_dd)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: model.friction_policy_summary.clone(),
        inspect: "Dashboard; Robustness Matrix; Candidate sheets".to_string(),
        notes: "Shows finalist sensitivity to fee, spread, and slippage stress scenarios."
            .to_string(),
    });

    let ranking = model.ranking_stability.as_ref();
    rows.push(ValidationFeatureRow {
        feature: "Ranking stability views".to_string(),
        scope: "Run".to_string(),
        status: ranking
            .filter(|summary| summary.finalist_k > 0)
            .map(|summary| {
                if summary.stable_finalist_count == summary.finalist_k {
                    "pass"
                } else if summary.stable_finalist_count > 0 {
                    "watch"
                } else {
                    "fail"
                }
                .to_string()
            })
            .unwrap_or_else(|| "not run".to_string()),
        evidence: ranking
            .map(|summary| {
                format!(
                    "stable finalists={}/{}",
                    summary.stable_finalist_count, summary.finalist_k
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        key_metrics: ranking
            .map(|summary| {
                format!(
                    "WF overlap={}/CSCV overlap={}/Cross overlap={}",
                    format_optional_usize(summary.wf_overlap),
                    format_optional_usize(summary.cscv_overlap),
                    format_optional_usize(summary.cross_overlap)
                )
            })
            .unwrap_or_else(|| "not run".to_string()),
        gate_or_policy: "Rank drift diagnostic across robust/WF/CSCV/cross views".to_string(),
        inspect: "Dashboard; Candidate sheets; Robustness Matrix".to_string(),
        notes: "Helps explain whether the same finalists remain strong across ranking views."
            .to_string(),
    });

    rows.push(ValidationFeatureRow {
        feature: "Execution route evidence".to_string(),
        scope: "Run".to_string(),
        status: if model.backend_route.contains("->CpuOnly")
            || model.actual_fallback_summary.trim() != "-"
        {
            "watch"
        } else {
            "observed"
        }
        .to_string(),
        evidence: format!(
            "backend={} route={}",
            if model.backend_used.is_empty() {
                "unknown"
            } else {
                model.backend_used.as_str()
            },
            if model.backend_route.is_empty() {
                "-"
            } else {
                model.backend_route.as_str()
            }
        ),
        key_metrics: format!(
            "disabled={} skipped={}",
            model.actual_disabled_module_summary, model.actual_skipped_module_summary
        ),
        gate_or_policy: model.expected_category_summary.clone(),
        inspect: "Dashboard".to_string(),
        notes: "Separates validation semantics from CPU/GPU route and auxiliary-tail behavior."
            .to_string(),
    });

    rows.push(ValidationFeatureRow {
        feature: "Validation evidence pack".to_string(),
        scope: "Run".to_string(),
        status: model
            .validation_evidence_pack
            .as_ref()
            .map(|pack| {
                if pack.status == "complete" {
                    "observed"
                } else {
                    "watch"
                }
                .to_string()
            })
            .unwrap_or_else(|| "not run".to_string()),
        evidence: model
            .validation_evidence_pack
            .as_ref()
            .map(|pack| {
                format!(
                    "schema={} version={} status={} workflow={}",
                    if pack.schema_version.is_empty() {
                        "-"
                    } else {
                        pack.schema_version.as_str()
                    },
                    pack.version
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    if pack.status.is_empty() {
                        "-"
                    } else {
                        pack.status.as_str()
                    },
                    if pack.workflow.is_empty() {
                        "-"
                    } else {
                        pack.workflow.as_str()
                    }
                )
            })
            .unwrap_or_else(|| "artifact did not include validation_evidence_pack".to_string()),
        key_metrics: model
            .validation_evidence_pack
            .as_ref()
            .map(|pack| {
                format!(
                    "requested={} evaluated={} retained={} approved={}",
                    format_optional_usize(pack.requested_candidates),
                    format_optional_usize(pack.evaluated_candidates),
                    format_optional_usize(pack.retained_candidates),
                    format_optional_usize(pack.approved_candidates)
                )
            })
            .unwrap_or_else(|| "not recorded".to_string()),
        gate_or_policy: model
            .validation_evidence_pack
            .as_ref()
            .map(|pack| {
                if pack.module_statuses.is_empty() {
                    "module availability not recorded".to_string()
                } else {
                    pack.module_statuses.clone()
                }
            })
            .unwrap_or_else(|| "legacy fallback".to_string()),
        inspect: "Dashboard; History; Report Guide".to_string(),
        notes: model
            .validation_evidence_pack
            .as_ref()
            .map(|pack| {
                if pack.metric_summaries.is_empty() {
                    pack.status_reason.clone()
                } else {
                    pack.metric_summaries.clone()
                }
            })
            .filter(|entry| !entry.trim().is_empty())
            .unwrap_or_else(|| {
                "Pack-backed evidence is additive; legacy run_verdict/provenance fallback remains available."
                    .to_string()
            }),
    });

    rows.push(ValidationFeatureRow {
        feature: "Validation provenance contract".to_string(),
        scope: "Run".to_string(),
        status: if model.validation_provenance.is_some() {
            "observed".to_string()
        } else {
            "not run".to_string()
        },
        evidence: model
            .validation_provenance
            .as_ref()
            .map(|provenance| provenance.stage_sequence.clone())
            .unwrap_or_else(|| "artifact did not include validation provenance".to_string()),
        key_metrics: model
            .validation_provenance
            .as_ref()
            .map(|provenance| {
                format!(
                    "score={} probability={} track={}",
                    provenance.score_channel,
                    provenance.probability_metric_source,
                    provenance.track_record_source
                )
            })
            .unwrap_or_else(|| "not recorded".to_string()),
        gate_or_policy: "Replay/provenance disclosure".to_string(),
        inspect: "Dashboard; Report Guide".to_string(),
        notes:
            "Useful for distinguishing modern full-contract artifacts from compatibility reports."
                .to_string(),
    });

    rows
}

fn build_validation_features_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
    compat_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Validation Features")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0x2563EB));
    apply_shareable_sheet_layout(sheet, "VectorGrid Validation Feature Coverage");
    let _ = sheet.set_freeze_panes(4, 0);
    let wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);
    for (col, width) in [
        (0u16, 30.0),
        (1u16, 16.0),
        (2u16, 14.0),
        (3u16, 42.0),
        (4u16, 40.0),
        (5u16, 42.0),
        (6u16, 34.0),
        (7u16, 46.0),
        (9u16, 24.0),
        (10u16, 10.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }

    sheet
        .merge_range(0, 0, 0, 7, "Validation Feature Coverage", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            7,
            "Run-by-run inventory of validation modules, derived diagnostics, gate status, and where to inspect the supporting rows.",
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let headers = [
        "Feature",
        "Scope",
        "Status",
        "Evidence",
        "Key Metrics",
        "Gate / Policy",
        "Where to Inspect",
        "Notes",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let rows = validation_feature_rows(model);
    let mut status_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for (idx, row) in rows.iter().enumerate() {
        let r = 4 + idx as u32;
        let _ = sheet.set_row_height(r, 42.0);
        *status_counts.entry(row.status.clone()).or_default() += 1;
        for (col, value) in [
            row.feature.as_str(),
            row.scope.as_str(),
            row.status.as_str(),
            row.evidence.as_str(),
            row.key_metrics.as_str(),
            row.gate_or_policy.as_str(),
            row.inspect.as_str(),
            row.notes.as_str(),
        ]
        .iter()
        .enumerate()
        {
            let fmt = if col == 2 {
                status_format(
                    row.status.as_str(),
                    text_fmt,
                    approved_fmt,
                    review_fmt,
                    rejected_fmt,
                    compat_fmt,
                )
            } else {
                &wrap_fmt
            };
            sheet
                .write_string_with_format(r, col as u16, excel_safe_text(value), fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    add_native_table(
        sheet,
        3,
        0,
        3 + rows.len() as u32,
        7,
        "ValidationFeaturesTable",
        &headers,
        TableStyle::Medium9,
    )?;

    sheet
        .write_string_with_format(3, 9, "Status", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .write_string_with_format(3, 10, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (idx, (status, count)) in status_counts.iter().enumerate() {
        let r = 4 + idx as u32;
        sheet
            .write_string_with_format(
                r,
                9,
                status,
                status_format(
                    status,
                    text_fmt,
                    approved_fmt,
                    review_fmt,
                    rejected_fmt,
                    compat_fmt,
                ),
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 10, *count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    if !status_counts.is_empty() {
        let last = 3 + status_counts.len() as u32;
        add_native_table(
            sheet,
            3,
            9,
            last,
            10,
            "ValidationFeatureStatusTable",
            &["Status", "Count"],
            TableStyle::Medium9,
        )?;
        let count_bar = ConditionalFormatDataBar::new().set_fill_color("5A8AC6");
        sheet
            .add_conditional_format(4, 10, last, 10, &count_bar)
            .map_err(|e: XlsxError| e.to_string())?;

        let mut status_chart = Chart::new(ChartType::Bar);
        status_chart
            .add_series()
            .set_categories(("Validation Features", 4, 9, last, 9))
            .set_values(("Validation Features", 4, 10, last, 10));
        status_chart
            .title()
            .set_name("Validation feature status counts");
        status_chart.legend().set_hidden();
        sheet
            .insert_chart_with_offset(3, 12, &status_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

struct DecisionLedgerRow {
    rank: usize,
    approval: String,
    module: &'static str,
    status: String,
    observed: String,
    required: String,
    margin: Option<f64>,
    detail: String,
    source: &'static str,
}

fn decision_row(
    row: &ExportCandidateRow,
    module: &'static str,
    status: impl Into<String>,
    observed: impl Into<String>,
    required: impl Into<String>,
    margin: Option<f64>,
    detail: impl Into<String>,
    source: &'static str,
) -> DecisionLedgerRow {
    DecisionLedgerRow {
        rank: row.rank,
        approval: display_approval_label(&row.approval_label).to_string(),
        module,
        status: status.into(),
        observed: observed.into(),
        required: required.into(),
        margin,
        detail: detail.into(),
        source,
    }
}

fn candidate_decision_rows(
    model: &ValidationExportModel,
    row: &ExportCandidateRow,
) -> Vec<DecisionLedgerRow> {
    let mut rows = Vec::new();

    let hard_gate_status = match row.hard_gate_passed {
        Some(true) => "pass",
        Some(false) => "fail",
        None if model.compatibility_mode => "compat",
        None => "not run",
    };
    rows.push(decision_row(
        row,
        "Hard Gate",
        hard_gate_status,
        match row.hard_gate_passed {
            Some(true) => "pass".to_string(),
            Some(false) => "fail".to_string(),
            None => "not recorded".to_string(),
        },
        "validation verdict pass".to_string(),
        None,
        primary_candidate_issue(row),
        "Candidate sheets",
    ));

    for (module, value, threshold, metric) in [
        ("PSR", row.psr, model.min_psr, "probabilistic Sharpe ratio"),
        ("DSR", row.dsr, model.min_dsr, "deflated Sharpe ratio"),
        (
            "CSCV Rank",
            row.cscv_oos_rank_frac_p50,
            model.min_cscv_oos_rank_frac,
            "median OOS rank fraction",
        ),
        (
            "CSCV Metric",
            row.cscv_oos_metric_p50,
            model.min_cscv_oos_metric,
            "median OOS metric",
        ),
    ] {
        let (status, required) = min_threshold_status(value, threshold);
        rows.push(decision_row(
            row,
            module,
            status,
            format_optional_number(value),
            required,
            threshold_gap(value, threshold),
            metric,
            "Rejection Diagnostics",
        ));
    }

    let track_status = match (row.track_record_bars, row.min_track_record_length_bars) {
        (Some(observed), Some(minimum)) if observed >= minimum => "pass",
        (Some(_), Some(_)) => "fail",
        (Some(_), None) => "observed",
        _ => "not run",
    };
    rows.push(decision_row(
        row,
        "Track Record",
        track_status,
        format!(
            "{}/{}",
            format_optional_usize(row.track_record_bars),
            format_optional_usize(row.min_track_record_length_bars)
        ),
        "observed >= minimum".to_string(),
        track_record_gap(row),
        "bars available for confidence checks",
        "Rejection Diagnostics",
    ));

    let walk_forward_has_result = row.wf_sharpe_median.is_some()
        || row.wf_dd_worst.is_some()
        || row.wf_pnl_efficiency.is_some()
        || row.wf_sharpe_efficiency.is_some();
    rows.push(decision_row(
        row,
        "Walk Forward",
        module_feature_status(model, "walk_forward", walk_forward_has_result),
        format!(
            "sharpe p50={} DD worst={}",
            format_optional_number(row.wf_sharpe_median),
            format_optional_number(row.wf_dd_worst)
        ),
        "diagnostic".to_string(),
        None,
        format!(
            "eff pnl={} sharpe={}",
            format_optional_number(row.wf_pnl_efficiency),
            format_optional_number(row.wf_sharpe_efficiency)
        ),
        "Dashboard / Distributions",
    ));

    let pbo_value = model
        .run_verdict
        .as_ref()
        .and_then(|verdict| verdict.pbo)
        .or_else(|| model.cscv_pbo.as_ref().and_then(|cscv| cscv.pbo));
    let pbo_threshold = model.max_pbo.or_else(|| {
        model
            .run_verdict
            .as_ref()
            .and_then(|verdict| verdict.max_pbo)
    });
    let (pbo_status, pbo_required) = max_threshold_status(pbo_value, pbo_threshold);
    rows.push(decision_row(
        row,
        "PBO",
        if model.cscv_pbo.is_some() {
            pbo_status
        } else {
            disabled_or_omitted_status(model, "cscv_pbo")
        },
        format_optional_number(pbo_value),
        pbo_required,
        max_threshold_margin(pbo_value, pbo_threshold),
        "run-level probability of backtest overfitting",
        "Dashboard / Robustness Matrix",
    ));

    let cross_status = if row.cross_score.is_some() || model.cross_ticker.is_some() {
        if row.cross_pareto {
            "pass".to_string()
        } else {
            "watch".to_string()
        }
    } else {
        disabled_or_omitted_status(model, "cross_ticker").to_string()
    };
    rows.push(decision_row(
        row,
        "Cross Dataset",
        cross_status,
        format!(
            "score={} pareto={}",
            format_optional_number(row.cross_score),
            if row.cross_pareto { "yes" } else { "no" }
        ),
        model
            .cross_ticker
            .as_ref()
            .map(|cross| display_cross_mode_label(&cross.mode).to_string())
            .unwrap_or_else(|| "not configured".to_string()),
        None,
        "generalization across comparison datasets",
        "Dashboard / Robustness Matrix",
    ));

    let bootstrap_has_result = row.bootstrap_prob_pnl_positive.is_some()
        || row.bootstrap_pnl_p05.is_some()
        || row.bootstrap_dd_p95.is_some()
        || row.bootstrap_stationary_pnl_p05.is_some()
        || row.bootstrap_stationary_dd_p95.is_some();
    rows.push(decision_row(
        row,
        "Bootstrap",
        module_feature_status(model, "trade_bootstrap", bootstrap_has_result),
        format!(
            "prob+={} iid p05={} stat p05={}",
            format_optional_number(row.bootstrap_prob_pnl_positive),
            format_optional_number(row.bootstrap_pnl_p05),
            format_optional_number(row.bootstrap_stationary_pnl_p05)
        ),
        model.bootstrap_mode_summary.clone(),
        None,
        bootstrap_mode_label(row),
        "Dashboard / Candidate sheets",
    ));

    let perturb_has_result = row.perturb_pnl_mean.is_some()
        || row.perturb_pnl_worst.is_some()
        || row.perturb_positive_frac.is_some();
    rows.push(decision_row(
        row,
        "Perturbation",
        module_feature_status(model, "perturb", perturb_has_result),
        format!(
            "mean={} worst={} positive={}",
            format_optional_number(row.perturb_pnl_mean),
            format_optional_number(row.perturb_pnl_worst),
            format_optional_number(row.perturb_positive_frac)
        ),
        "diagnostic".to_string(),
        None,
        "randomized parameter deviation robustness",
        "Candidate sheets",
    ));

    rows.push(decision_row(
        row,
        "Plateau",
        if row.plateau_score.is_some() {
            "observed"
        } else {
            "not run"
        },
        format_optional_number(row.plateau_score),
        "derived report diagnostic".to_string(),
        None,
        plateau_summary_label(row),
        "Dashboard / Candidate sheets",
    ));

    let regime_has_result = !row.regime_bucket_rows.is_empty()
        || row.regime_stable_bucket_frac.is_some()
        || row.regime_worst_sharpe.is_some();
    rows.push(decision_row(
        row,
        "Regime",
        if regime_has_result {
            regime_status(&row.regime_fragility_flag)
        } else {
            "not run"
        },
        format!(
            "flag={} stable={}",
            display_regime_flag(&row.regime_fragility_flag),
            format_optional_number(row.regime_stable_bucket_frac)
        ),
        "descriptive stability".to_string(),
        None,
        format!(
            "worst={} sharpe={} maxdd={}",
            if row.regime_worst_bucket.is_empty() {
                "not run"
            } else {
                row.regime_worst_bucket.as_str()
            },
            format_optional_number(row.regime_worst_sharpe),
            format_optional_number(row.regime_worst_max_dd)
        ),
        "Regime Audit",
    ));

    let friction_has_result = row.friction_pass_frac.is_some()
        || row.friction_worst_sharpe.is_some()
        || row.friction_worst_max_dd.is_some()
        || row.friction_max_rank_drift.is_some();
    rows.push(decision_row(
        row,
        "Friction",
        if friction_has_result {
            friction_status(&row.friction_flag)
        } else {
            "not run"
        },
        format!(
            "pass frac={} drift={}",
            format_optional_number(row.friction_pass_frac),
            format_optional_usize(row.friction_max_rank_drift)
        ),
        "pnl >= 0 and sharpe >= 0 under stress".to_string(),
        None,
        if row.friction_worst_scenario.is_empty() {
            model.friction_policy_summary.clone()
        } else {
            format!("worst scenario={}", row.friction_worst_scenario)
        },
        "Dashboard / Candidate sheets",
    ));

    rows
}

fn status_bucket(status: &str) -> &'static str {
    match status {
        "pass" | "completed" => "Strong",
        "fail" => "Fail",
        "watch" | "observed" => "Watch / Observed",
        _ => "Unavailable",
    }
}

fn build_candidate_decision_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
    compat_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Decision Ledger")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(105);
    sheet.set_tab_color(Color::RGB(0x7C3AED));
    apply_shareable_sheet_layout(sheet, "VectorGrid Candidate Decision Ledger");
    let _ = sheet.set_freeze_panes(4, 2);
    let detail_wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);
    for (col, width) in [
        (0u16, 8.0),
        (1u16, 16.0),
        (2u16, 18.0),
        (3u16, 13.0),
        (4u16, 24.0),
        (5u16, 26.0),
        (6u16, 14.0),
        (7u16, 40.0),
        (8u16, 30.0),
        (9u16, 13.0),
        (10u16, 13.0),
        (11u16, 13.0),
        (12u16, 13.0),
        (13u16, 12.0),
        (14u16, 12.0),
        (16u16, 14.0),
        (17u16, 14.0),
        (18u16, 14.0),
        (19u16, 14.0),
        (20u16, 14.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }

    sheet
        .merge_range(0, 0, 0, 14, "Candidate Decision Map", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            14,
            "Each cell shows whether a validation module passed, failed, was observed, or was unavailable for the candidate.",
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let heatmap_headers = [
        "Rank",
        "Approval",
        "Hard Gate",
        "PSR",
        "DSR",
        "Track",
        "Walk Fwd",
        "CSCV Rank",
        "CSCV Metric",
        "PBO",
        "Cross",
        "Bootstrap",
        "Perturb",
        "Plateau",
        "Regime",
        "Friction",
    ];
    for (col, header) in heatmap_headers.iter().enumerate() {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let mut all_decisions = Vec::new();
    let mut status_summary_rows: Vec<(usize, usize, usize, usize, usize)> = Vec::new();
    for (idx, candidate) in model.all_rows.iter().enumerate() {
        let decisions = candidate_decision_rows(model, candidate);
        let r = 4 + idx as u32;
        sheet
            .write_number_with_format(r, 0, candidate.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                1,
                display_approval_label(&candidate.approval_label),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let mut strong = 0usize;
        let mut watch = 0usize;
        let mut fail = 0usize;
        let mut unavailable = 0usize;
        for (offset, decision) in decisions.iter().enumerate() {
            match status_bucket(&decision.status) {
                "Strong" => strong += 1,
                "Fail" => fail += 1,
                "Watch / Observed" => watch += 1,
                _ => unavailable += 1,
            }
            sheet
                .write_string_with_format(
                    r,
                    2 + offset as u16,
                    &decision.status,
                    status_format(
                        &decision.status,
                        text_fmt,
                        approved_fmt,
                        review_fmt,
                        rejected_fmt,
                        compat_fmt,
                    ),
                )
                .map_err(|e: XlsxError| e.to_string())?;
        }
        status_summary_rows.push((candidate.rank, strong, watch, fail, unavailable));
        all_decisions.extend(decisions);
    }

    if !model.all_rows.is_empty() {
        let heatmap_last = 3 + model.all_rows.len() as u32;
        add_native_table(
            sheet,
            3,
            0,
            heatmap_last,
            15,
            "CandidateDecisionMapTable",
            &heatmap_headers,
            TableStyle::Medium5,
        )?;
    } else {
        sheet
            .write_string_with_format(
                4,
                0,
                "No candidate rows present in this artifact.",
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let summary_headers = ["Rank", "Strong", "Watch / Observed", "Fail", "Unavailable"];
    for (col, header) in summary_headers.iter().enumerate() {
        sheet
            .write_string_with_format(3, 17 + col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (idx, (rank, strong, watch, fail, unavailable)) in status_summary_rows.iter().enumerate() {
        let r = 4 + idx as u32;
        for (col, value) in [*rank, *strong, *watch, *fail, *unavailable]
            .iter()
            .enumerate()
        {
            sheet
                .write_number_with_format(r, 17 + col as u16, *value as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }
    if !status_summary_rows.is_empty() {
        let summary_last = 3 + status_summary_rows.len() as u32;
        add_native_table(
            sheet,
            3,
            17,
            summary_last,
            21,
            "CandidateDecisionStatusTable",
            &summary_headers,
            TableStyle::Medium5,
        )?;
        let mut chart = Chart::new(ChartType::BarStacked);
        for (name, col) in [
            ("Strong", 18u16),
            ("Watch / Observed", 19u16),
            ("Fail", 20u16),
            ("Unavailable", 21u16),
        ] {
            chart
                .add_series()
                .set_name(name)
                .set_categories(("Decision Ledger", 4, 17, summary_last, 17))
                .set_values(("Decision Ledger", 4, col, summary_last, col));
        }
        chart.title().set_name("Validation outcomes by candidate");
        chart.x_axis().set_name("Module count");
        chart.y_axis().set_name("Rank");
        sheet
            .insert_chart_with_offset(3, 23, &chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let ledger_header_row = 7 + model.all_rows.len() as u32;
    sheet
        .merge_range(
            ledger_header_row - 2,
            0,
            ledger_header_row - 2,
            8,
            "Decision Ledger Detail",
            title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let detail_headers = [
        "Rank", "Approval", "Module", "Status", "Observed", "Required", "Margin", "Detail",
        "Source",
    ];
    for (col, header) in detail_headers.iter().enumerate() {
        sheet
            .write_string_with_format(ledger_header_row, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (idx, decision) in all_decisions.iter().enumerate() {
        let r = ledger_header_row + 1 + idx as u32;
        let _ = sheet.set_row_height(r, 30.0);
        sheet
            .write_number_with_format(r, 0, decision.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 1, excel_safe_text(&decision.approval), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 2, decision.module, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                3,
                excel_safe_text(&decision.status),
                status_format(
                    &decision.status,
                    text_fmt,
                    approved_fmt,
                    review_fmt,
                    rejected_fmt,
                    compat_fmt,
                ),
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 4, excel_safe_text(&decision.observed), &detail_wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 5, excel_safe_text(&decision.required), &detail_wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 6, decision.margin, num_fmt)?;
        sheet
            .write_string_with_format(r, 7, excel_safe_text(&decision.detail), &detail_wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 8, decision.source, &detail_wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    if !all_decisions.is_empty() {
        let ledger_last = ledger_header_row + all_decisions.len() as u32;
        add_native_table(
            sheet,
            ledger_header_row,
            0,
            ledger_last,
            8,
            "CandidateDecisionLedgerTable",
            &detail_headers,
            TableStyle::Medium5,
        )?;
        apply_zero_centered_formatting(sheet, ledger_header_row + 1, 6, ledger_last, 6)?;
    }

    Ok(())
}

fn worst_drawdown_source(row: &ExportCandidateRow) -> (Option<f64>, &'static str) {
    [
        (Some(row.max_dd), "In-sample MaxDD"),
        (row.wf_dd_worst, "WF Worst DD"),
        (row.bootstrap_dd_p95, "Bootstrap DD95"),
        (row.bootstrap_stationary_dd_p95, "Stationary DD95"),
        (row.regime_worst_max_dd, "Regime Worst DD"),
        (row.friction_worst_max_dd, "Friction Worst DD"),
    ]
    .into_iter()
    .filter_map(|(value, source)| value.filter(|v| v.is_finite()).map(|v| (v, source)))
    .max_by(|(a, _), (b, _)| a.total_cmp(b))
    .map(|(value, source)| (Some(value), source))
    .unwrap_or((None, "not run"))
}

fn worst_negative_pnl_source(row: &ExportCandidateRow) -> (Option<f64>, &'static str) {
    [
        (row.bootstrap_pnl_p05, "Bootstrap P05"),
        (row.bootstrap_stationary_pnl_p05, "Stationary P05"),
        (row.perturb_pnl_worst, "Perturb Worst"),
    ]
    .into_iter()
    .filter_map(|(value, source)| value.filter(|v| v.is_finite()).map(|v| (v, source)))
    .min_by(|(a, _), (b, _)| a.total_cmp(b))
    .map(|(value, source)| (Some(value), source))
    .unwrap_or((None, "not run"))
}

fn build_downside_stress_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Downside Stress")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0xDC2626));
    apply_shareable_sheet_layout(sheet, "VectorGrid Downside Stress");
    let _ = sheet.set_freeze_panes(4, 0);
    for (col, width) in [
        (0u16, 8.0),
        (1u16, 15.0),
        (2u16, 13.0),
        (3u16, 13.0),
        (4u16, 15.0),
        (5u16, 15.0),
        (6u16, 15.0),
        (7u16, 15.0),
        (8u16, 13.0),
        (9u16, 20.0),
        (10u16, 14.0),
        (11u16, 20.0),
        (12u16, 15.0),
        (13u16, 15.0),
        (14u16, 15.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }

    sheet
        .merge_range(0, 0, 0, 14, "Downside Stress Summary", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            14,
            "Compares the worst drawdown and negative-PnL evidence from in-sample, walk-forward, bootstrap, regime, friction, and perturbation checks.",
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let headers = [
        "Rank",
        "Approval",
        "MaxDD",
        "WF DD Worst",
        "Bootstrap DD95",
        "Stat DD95",
        "Regime DD",
        "Friction DD",
        "Worst DD",
        "Worst DD Source",
        "Worst PnL",
        "Worst PnL Source",
        "Regime Sharpe",
        "Friction Sharpe",
        "Perturb Positive",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    for (idx, row) in model.all_rows.iter().enumerate() {
        let r = 4 + idx as u32;
        let (worst_dd, worst_dd_source) = worst_drawdown_source(row);
        let (worst_pnl, worst_pnl_source) = worst_negative_pnl_source(row);
        sheet
            .write_number_with_format(r, 0, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 1, display_approval_label(&row.approval_label), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 2, Some(row.max_dd), pct_fmt)?;
        write_optional_number(sheet, r, 3, row.wf_dd_worst, pct_fmt)?;
        write_optional_number(sheet, r, 4, row.bootstrap_dd_p95, pct_fmt)?;
        write_optional_number(sheet, r, 5, row.bootstrap_stationary_dd_p95, pct_fmt)?;
        write_optional_number(sheet, r, 6, row.regime_worst_max_dd, pct_fmt)?;
        write_optional_number(sheet, r, 7, row.friction_worst_max_dd, pct_fmt)?;
        write_optional_number(sheet, r, 8, worst_dd, pct_fmt)?;
        sheet
            .write_string_with_format(r, 9, worst_dd_source, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 10, worst_pnl, num_fmt)?;
        sheet
            .write_string_with_format(r, 11, worst_pnl_source, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 12, row.regime_worst_sharpe, num_fmt)?;
        write_optional_number(sheet, r, 13, row.friction_worst_sharpe, num_fmt)?;
        write_optional_number(sheet, r, 14, row.perturb_positive_frac, pct_fmt)?;
    }

    if !model.all_rows.is_empty() {
        let row_min = 4u32;
        let row_max = 3 + model.all_rows.len() as u32;
        add_native_table(
            sheet,
            3,
            0,
            row_max,
            14,
            "DownsideStressTable",
            &headers,
            TableStyle::Medium3,
        )?;
        let bad_scale = ConditionalFormat3ColorScale::new()
            .set_minimum_color("63BE7B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("F8696B");
        for col in 2u16..=8u16 {
            sheet
                .add_conditional_format(row_min, col, row_max, col, &bad_scale)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        for col in [10u16, 12u16, 13u16] {
            apply_zero_centered_formatting(sheet, row_min, col, row_max, col)?;
        }
        let probability_bar = probability_data_bar("5A8AC6");
        sheet
            .add_conditional_format(row_min, 14, row_max, 14, &probability_bar)
            .map_err(|e: XlsxError| e.to_string())?;

        let mut worst_chart = Chart::new(ChartType::Column);
        worst_chart
            .add_series()
            .set_categories(("Downside Stress", row_min, 0, row_max, 0))
            .set_values(("Downside Stress", row_min, 8, row_max, 8));
        worst_chart.title().set_name("Worst drawdown by candidate");
        worst_chart.x_axis().set_name("Rank");
        worst_chart
            .y_axis()
            .set_name("Drawdown")
            .set_num_format("0.0%");
        worst_chart.legend().set_hidden();
        sheet
            .insert_chart_with_offset(3, 16, &worst_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;

        let component_series = [
            (
                "WF Worst",
                3u16,
                model.all_rows.iter().all(|row| row.wf_dd_worst.is_some()),
            ),
            (
                "Bootstrap DD95",
                4u16,
                model
                    .all_rows
                    .iter()
                    .all(|row| row.bootstrap_dd_p95.is_some()),
            ),
            (
                "Stationary DD95",
                5u16,
                model
                    .all_rows
                    .iter()
                    .all(|row| row.bootstrap_stationary_dd_p95.is_some()),
            ),
            (
                "Regime DD",
                6u16,
                model
                    .all_rows
                    .iter()
                    .all(|row| row.regime_worst_max_dd.is_some()),
            ),
            (
                "Friction DD",
                7u16,
                model
                    .all_rows
                    .iter()
                    .all(|row| row.friction_worst_max_dd.is_some()),
            ),
        ];
        if component_series.iter().any(|(_, _, available)| *available) {
            let mut component_chart = Chart::new(ChartType::Column);
            component_chart
                .add_series()
                .set_name("MaxDD")
                .set_categories(("Downside Stress", row_min, 0, row_max, 0))
                .set_values(("Downside Stress", row_min, 2, row_max, 2));
            for (name, col, available) in component_series {
                if !available {
                    continue;
                }
                component_chart
                    .add_series()
                    .set_name(name)
                    .set_categories(("Downside Stress", row_min, 0, row_max, 0))
                    .set_values(("Downside Stress", row_min, col, row_max, col));
            }
            component_chart
                .title()
                .set_name("Drawdown stress components by candidate");
            component_chart.x_axis().set_name("Rank");
            component_chart
                .y_axis()
                .set_name("Drawdown")
                .set_num_format("0.0%");
            sheet
                .insert_chart_with_offset(20, 16, &component_chart, 8, 8)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    } else {
        sheet
            .write_string_with_format(
                4,
                0,
                "No candidate rows present in this artifact.",
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

fn build_robustness_matrix(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
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
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Robustness Matrix")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0xA855F7));
    apply_shareable_sheet_layout(sheet, "VectorGrid Robustness Matrix");
    let _ = sheet.set_freeze_panes(4, 0);
    let wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);
    for (col, width) in [
        (0u16, 22.0),
        (1u16, 20.0),
        (2u16, 20.0),
        (3u16, 14.0),
        (4u16, 44.0),
        (6u16, 13.0),
        (7u16, 13.0),
        (8u16, 13.0),
        (9u16, 13.0),
        (10u16, 13.0),
        (11u16, 13.0),
        (12u16, 13.0),
        (13u16, 13.0),
        (14u16, 13.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }

    sheet
        .merge_range(0, 0, 0, 4, "Top Candidate Robustness Matrix", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            4,
            "Derived from the saved validation artifact; status uses explicit report gates where available and otherwise marks metrics as observed.",
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    for (col, header) in ["Dimension", "Observed", "Reference", "Status", "Readout"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let top_candidate = model
        .approved_rows
        .first()
        .or_else(|| model.shortlist_rows.first())
        .or_else(|| model.all_rows.first());
    let run_verdict = model.run_verdict.as_ref();
    let recommendation = run_verdict
        .map(|verdict| display_recommendation_label(&verdict.final_recommendation))
        .unwrap_or_else(|| "legacy compatibility mode".to_string());
    let run_status = if run_verdict.map(|verdict| verdict.approved).unwrap_or(false) {
        "pass"
    } else if run_verdict.is_some() {
        "fail"
    } else {
        "observed"
    };

    let mut dimensions: Vec<(String, String, String, String, String)> = Vec::new();
    dimensions.push((
        "Run verdict".to_string(),
        run_verdict
            .map(|verdict| {
                if verdict.approved {
                    "approved".to_string()
                } else if verdict.passed {
                    "passed, not approved".to_string()
                } else {
                    "failed".to_string()
                }
            })
            .unwrap_or_else(|| "compatibility artifact".to_string()),
        "approved run".to_string(),
        run_status.to_string(),
        recommendation,
    ));

    if let Some(row) = top_candidate {
        let hard_gate_status = match row.hard_gate_passed {
            Some(true) => "pass",
            Some(false) => "fail",
            None => "observed",
        };
        dimensions.push((
            "Candidate hard gate".to_string(),
            match row.hard_gate_passed {
                Some(true) => "pass".to_string(),
                Some(false) => "fail".to_string(),
                None => "compatibility / not recorded".to_string(),
            },
            "must pass hard gates".to_string(),
            hard_gate_status.to_string(),
            row.approval_reason.clone(),
        ));

        for (label, observed, threshold, readout) in [
            ("PSR", row.psr, model.min_psr, "probabilistic Sharpe ratio"),
            ("DSR", row.dsr, model.min_dsr, "deflated Sharpe ratio"),
            (
                "CSCV OOS rank",
                row.cscv_oos_rank_frac_p50,
                model.min_cscv_oos_rank_frac,
                "median out-of-sample rank fraction",
            ),
            (
                "CSCV OOS metric",
                row.cscv_oos_metric_p50,
                model.min_cscv_oos_metric,
                "median out-of-sample metric",
            ),
        ] {
            let (status, reference) = min_threshold_status(observed, threshold);
            dimensions.push((
                label.to_string(),
                format_optional_number(observed),
                reference,
                status.to_string(),
                readout.to_string(),
            ));
        }

        let track_status = match (row.track_record_bars, row.min_track_record_length_bars) {
            (Some(observed), Some(minimum)) if observed >= minimum => "pass",
            (Some(_), Some(_)) => "fail",
            (Some(_), None) => "observed",
            _ => "not run",
        };
        dimensions.push((
            "Track record".to_string(),
            format!(
                "{}/{}",
                format_optional_usize(row.track_record_bars),
                format_optional_usize(row.min_track_record_length_bars)
            ),
            "observed >= minimum".to_string(),
            track_status.to_string(),
            "bars used for statistical confidence checks".to_string(),
        ));

        let pbo_value = run_verdict
            .and_then(|verdict| verdict.pbo)
            .or_else(|| model.cscv_pbo.as_ref().and_then(|cscv| cscv.pbo));
        let pbo_threshold = model
            .max_pbo
            .or_else(|| run_verdict.and_then(|verdict| verdict.max_pbo));
        let (pbo_status, pbo_reference) = max_threshold_status(pbo_value, pbo_threshold);
        dimensions.push((
            "CSCV PBO".to_string(),
            format_optional_number(pbo_value),
            pbo_reference,
            pbo_status.to_string(),
            "probability of backtest overfitting".to_string(),
        ));

        let cross_status = if row.cross_score.is_none() && model.cross_ticker.is_none() {
            "not run"
        } else if row.cross_pareto {
            "pass"
        } else {
            "watch"
        };
        dimensions.push((
            "Cross-ticker".to_string(),
            format!(
                "score={} pareto={}",
                format_optional_number(row.cross_score),
                if row.cross_pareto { "yes" } else { "no" }
            ),
            model
                .cross_ticker
                .as_ref()
                .map(|cross| display_cross_mode_label(&cross.mode).to_string())
                .unwrap_or_else(|| "not configured".to_string()),
            cross_status.to_string(),
            "generalization across configured comparison datasets".to_string(),
        ));

        dimensions.push((
            "Bootstrap positive".to_string(),
            format_optional_number(row.bootstrap_prob_pnl_positive),
            "observed; higher is better".to_string(),
            if row.bootstrap_prob_pnl_positive.is_some() {
                "observed"
            } else {
                "not run"
            }
            .to_string(),
            "closed-trade resampling probability of positive PnL".to_string(),
        ));
        dimensions.push((
            "Bootstrap downside".to_string(),
            format!(
                "iid p05={} stat p05={}",
                format_optional_number(row.bootstrap_pnl_p05),
                format_optional_number(row.bootstrap_stationary_pnl_p05)
            ),
            "observed; less negative is better".to_string(),
            if row.bootstrap_pnl_p05.is_some() || row.bootstrap_stationary_pnl_p05.is_some() {
                "observed"
            } else {
                "not run"
            }
            .to_string(),
            bootstrap_mode_label(row).to_string(),
        ));
        dimensions.push((
            "Walk-forward efficiency".to_string(),
            format!(
                "pnl={} sharpe={}",
                format_optional_number(row.wf_pnl_efficiency),
                format_optional_number(row.wf_sharpe_efficiency)
            ),
            "observed; closer to 1 is stronger".to_string(),
            if row.wf_pnl_efficiency.is_some() || row.wf_sharpe_efficiency.is_some() {
                "observed"
            } else {
                "not run"
            }
            .to_string(),
            model.wfe_policy_summary.clone(),
        ));
        dimensions.push((
            "Regime stability".to_string(),
            format!(
                "{} | stable frac={}",
                display_regime_flag(&row.regime_fragility_flag),
                format_optional_number(row.regime_stable_bucket_frac)
            ),
            display_regime_support(&row.regime_support_label).to_string(),
            regime_status(&row.regime_fragility_flag).to_string(),
            format!(
                "worst bucket={} sharpe={} maxdd={}",
                if row.regime_worst_bucket.is_empty() {
                    "not run"
                } else {
                    row.regime_worst_bucket.as_str()
                },
                format_optional_number(row.regime_worst_sharpe),
                format_optional_number(row.regime_worst_max_dd)
            ),
        ));
        dimensions.push((
            "Friction stress".to_string(),
            format!(
                "pass frac={} drift={}",
                format_optional_number(row.friction_pass_frac),
                format_optional_usize(row.friction_max_rank_drift)
            ),
            "fee/spread/slippage stress".to_string(),
            friction_status(&row.friction_flag).to_string(),
            if row.friction_worst_scenario.is_empty() {
                model.friction_policy_summary.clone()
            } else {
                format!("worst scenario={}", row.friction_worst_scenario)
            },
        ));
        dimensions.push((
            "Plateau".to_string(),
            format_optional_number(row.plateau_score),
            "observed neighborhood robustness".to_string(),
            if row.plateau_score.is_some() {
                "observed"
            } else {
                "not run"
            }
            .to_string(),
            plateau_summary_label(row),
        ));
    } else {
        dimensions.push((
            "Candidate availability".to_string(),
            "no candidates".to_string(),
            "validation result candidates".to_string(),
            "fail".to_string(),
            "No candidate rows were present in this artifact.".to_string(),
        ));
    }

    for (idx, (dimension, observed, reference, status, readout)) in dimensions.iter().enumerate() {
        let row = 4 + idx as u32;
        let _ = sheet.set_row_height(row, 30.0);
        sheet
            .write_string_with_format(row, 0, excel_safe_text(dimension), label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 1, excel_safe_text(observed), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 2, excel_safe_text(reference), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                row,
                3,
                status,
                status_format(
                    status,
                    text_fmt,
                    approved_fmt,
                    review_fmt,
                    rejected_fmt,
                    compat_fmt,
                ),
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 4, excel_safe_text(readout), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let peer_header_row = 6 + dimensions.len() as u32;
    sheet
        .write_string_with_format(peer_header_row - 2, 0, "Peer Comparison", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (col, header) in [
        "Rank",
        "Approval",
        "Robust",
        "PnL",
        "Sharpe",
        "MaxDD",
        "PSR",
        "DSR",
        "CSCV Rank",
        "Cross",
        "Boot Prob+",
        "WFE Sharpe",
        "Regime",
        "Friction",
        "Max Drift",
    ]
    .iter()
    .enumerate()
    {
        sheet
            .write_string_with_format(peer_header_row, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let mut peer_rows = Vec::new();
    let mut seen_ranks = std::collections::BTreeSet::new();
    for row in &model.approved_rows {
        if seen_ranks.insert(row.rank) {
            peer_rows.push(row);
        }
        if peer_rows.len() >= 12 {
            break;
        }
    }
    if peer_rows.len() < 12 {
        for row in &model.shortlist_rows {
            if seen_ranks.insert(row.rank) {
                peer_rows.push(row);
            }
            if peer_rows.len() >= 12 {
                break;
            }
        }
    }
    if peer_rows.len() < 12 {
        for row in &model.all_rows {
            if seen_ranks.insert(row.rank) {
                peer_rows.push(row);
            }
            if peer_rows.len() >= 12 {
                break;
            }
        }
    }

    for (idx, row) in peer_rows.iter().enumerate() {
        let r = peer_header_row + 1 + idx as u32;
        let status_fmt = match row.approval_label.as_str() {
            "approved" => approved_fmt,
            "rejected" => rejected_fmt,
            "compat_shortlist" | "compat_context" => compat_fmt,
            _ => review_fmt,
        };
        sheet
            .write_number_with_format(r, 0, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                1,
                display_approval_label(&row.approval_label),
                status_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 2, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 3, row.pnl, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 4, row.sharpe, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 5, row.max_dd, pct_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 6, row.psr, pct_fmt)?;
        write_optional_number(sheet, r, 7, row.dsr, pct_fmt)?;
        write_optional_number(sheet, r, 8, row.cscv_oos_rank_frac_p50, pct_fmt)?;
        write_optional_number(sheet, r, 9, row.cross_score, num_fmt)?;
        write_optional_number(sheet, r, 10, row.bootstrap_prob_pnl_positive, pct_fmt)?;
        write_optional_number(sheet, r, 11, row.wf_sharpe_efficiency, num_fmt)?;
        sheet
            .write_string_with_format(
                r,
                12,
                display_regime_flag(&row.regime_fragility_flag),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                13,
                if row.friction_flag.is_empty() {
                    "not run"
                } else {
                    row.friction_flag.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(sheet, r, 14, row.max_view_rank_drift, int_fmt)?;
    }

    if !peer_rows.is_empty() {
        let row_min = peer_header_row + 1;
        let row_max = peer_header_row + peer_rows.len() as u32;
        add_native_table(
            sheet,
            peer_header_row,
            0,
            row_max,
            14,
            "RobustnessPeerTable",
            &[
                "Rank",
                "Approval",
                "Robust",
                "PnL",
                "Sharpe",
                "MaxDD",
                "PSR",
                "DSR",
                "CSCV Rank",
                "Cross",
                "Boot Prob+",
                "WFE Sharpe",
                "Regime",
                "Friction",
                "Max Drift",
            ],
            TableStyle::Medium6,
        )?;

        let score_bar = ConditionalFormatDataBar::new().set_fill_color("63BE7B");
        let good_scale = ConditionalFormat3ColorScale::new()
            .set_minimum_color("F8696B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("63BE7B");
        let bad_scale = ConditionalFormat3ColorScale::new()
            .set_minimum_color("63BE7B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("F8696B");
        for col in [2u16, 6u16, 7u16, 8u16, 9u16, 10u16, 11u16] {
            sheet
                .add_conditional_format(row_min, col, row_max, col, &score_bar)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        for col in [3u16, 4u16] {
            sheet
                .add_conditional_format(row_min, col, row_max, col, &good_scale)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        for col in [5u16, 14u16] {
            sheet
                .add_conditional_format(row_min, col, row_max, col, &bad_scale)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        let mut score_chart = Chart::new(ChartType::Column);
        score_chart
            .add_series()
            .set_categories(("Robustness Matrix", row_min, 0, row_max, 0))
            .set_values(("Robustness Matrix", row_min, 2, row_max, 2));
        score_chart.title().set_name("Peer robust score by rank");
        score_chart.x_axis().set_name("Rank");
        score_chart.y_axis().set_name("Robust score");
        score_chart.legend().set_hidden();
        sheet
            .insert_chart_with_offset(3, 7, &score_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;

        let mut probability_chart = Chart::new(ChartType::Line);
        let mut probability_series_count = 0usize;
        for (name, col, available) in [
            ("PSR", 6u16, peer_rows.iter().all(|row| row.psr.is_some())),
            ("DSR", 7u16, peer_rows.iter().all(|row| row.dsr.is_some())),
            (
                "CSCV Rank",
                8u16,
                peer_rows
                    .iter()
                    .all(|row| row.cscv_oos_rank_frac_p50.is_some()),
            ),
            (
                "Bootstrap Prob+",
                10u16,
                peer_rows
                    .iter()
                    .all(|row| row.bootstrap_prob_pnl_positive.is_some()),
            ),
            (
                "WFE Sharpe",
                11u16,
                peer_rows
                    .iter()
                    .all(|row| row.wf_sharpe_efficiency.is_some()),
            ),
        ] {
            if available {
                probability_series_count += 1;
                probability_chart
                    .add_series()
                    .set_name(name)
                    .set_categories(("Robustness Matrix", row_min, 0, row_max, 0))
                    .set_values(("Robustness Matrix", row_min, col, row_max, col));
            }
        }
        if probability_series_count > 0 {
            probability_chart
                .title()
                .set_name("Validation ratios and probabilities by rank");
            probability_chart.x_axis().set_name("Rank");
            probability_chart.y_axis().set_name("Metric");
            sheet
                .insert_chart_with_offset(peer_header_row, 16, &probability_chart, 8, 8)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    } else {
        sheet
            .write_string_with_format(
                peer_header_row + 1,
                0,
                "No candidate rows available for peer comparison.",
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

fn threshold_gap(value: Option<f64>, threshold: Option<f64>) -> Option<f64> {
    match (value.filter(|v| v.is_finite()), threshold) {
        (Some(value), Some(threshold)) => Some(value - threshold),
        _ => None,
    }
}

fn max_threshold_margin(value: Option<f64>, threshold: Option<f64>) -> Option<f64> {
    match (value.filter(|v| v.is_finite()), threshold) {
        (Some(value), Some(threshold)) => Some(threshold - value),
        _ => None,
    }
}

fn track_record_gap(row: &ExportCandidateRow) -> Option<f64> {
    match (row.track_record_bars, row.min_track_record_length_bars) {
        (Some(observed), Some(minimum)) => Some(observed as f64 - minimum as f64),
        _ => None,
    }
}

fn primary_candidate_issue(row: &ExportCandidateRow) -> String {
    if !row.fail_reasons.trim().is_empty() {
        row.fail_reasons.clone()
    } else if !row.warning_flags.trim().is_empty() {
        row.warning_flags.clone()
    } else if !row.approval_reason.trim().is_empty() {
        row.approval_reason.clone()
    } else {
        "-".to_string()
    }
}

fn observed_gate_failure_count(gaps: &[Option<f64>]) -> usize {
    gaps.iter()
        .flatten()
        .copied()
        .filter(|value| value.is_finite() && *value < 0.0)
        .count()
}

fn run_pbo_margin(model: &ValidationExportModel) -> Option<f64> {
    let pbo_value = model
        .run_verdict
        .as_ref()
        .and_then(|verdict| verdict.pbo)
        .or_else(|| model.cscv_pbo.as_ref().and_then(|cscv| cscv.pbo));
    let pbo_threshold = model.max_pbo.or_else(|| {
        model
            .run_verdict
            .as_ref()
            .and_then(|verdict| verdict.max_pbo)
    });
    max_threshold_margin(pbo_value, pbo_threshold)
}

fn diagnostic_rows(model: &ValidationExportModel) -> Vec<&ExportCandidateRow> {
    let mut rows: Vec<&ExportCandidateRow> = model
        .all_rows
        .iter()
        .filter(|row| {
            row.approval_label != "approved"
                || !row.fail_reasons.trim().is_empty()
                || !row.warning_flags.trim().is_empty()
        })
        .collect();
    if rows.is_empty() {
        rows.extend(model.all_rows.iter());
    }
    rows.sort_by(|a, b| {
        let a_failures = observed_gate_failure_count(&[
            threshold_gap(a.psr, model.min_psr),
            threshold_gap(a.dsr, model.min_dsr),
            threshold_gap(a.cscv_oos_rank_frac_p50, model.min_cscv_oos_rank_frac),
            threshold_gap(a.cscv_oos_metric_p50, model.min_cscv_oos_metric),
            track_record_gap(a),
        ]);
        let b_failures = observed_gate_failure_count(&[
            threshold_gap(b.psr, model.min_psr),
            threshold_gap(b.dsr, model.min_dsr),
            threshold_gap(b.cscv_oos_rank_frac_p50, model.min_cscv_oos_rank_frac),
            threshold_gap(b.cscv_oos_metric_p50, model.min_cscv_oos_metric),
            track_record_gap(b),
        ]);
        b_failures
            .cmp(&a_failures)
            .then_with(|| a.rank.cmp(&b.rank))
    });
    rows
}

fn build_rejection_diagnostics(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Rejection Diagnostics")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0xEF4444));
    apply_shareable_sheet_layout(sheet, "VectorGrid Rejection Diagnostics");
    let _ = sheet.set_freeze_panes(4, 3);
    let wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);
    for (col, width) in [
        (0u16, 38.0),
        (1u16, 10.0),
        (3u16, 8.0),
        (4u16, 14.0),
        (5u16, 36.0),
        (6u16, 12.0),
        (7u16, 12.0),
        (8u16, 14.0),
        (9u16, 14.0),
        (10u16, 13.0),
        (11u16, 13.0),
        (12u16, 20.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }

    sheet
        .merge_range(0, 0, 0, 12, "Rejection Diagnostics", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            12,
            "Gate distance columns show observed minus required minimum; positive values pass. PBO margin is max allowed PBO minus observed PBO. Observed Gate Fails counts negative recorded margins and excludes unavailable gates.",
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    sheet
        .write_string_with_format(3, 0, "Rejection / Review Reason", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .write_string_with_format(3, 1, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let reason_counts = top_reason_counts(model, 12);
    if reason_counts.is_empty() {
        sheet
            .write_string_with_format(4, 0, "No rejected or review rows", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        for (idx, (reason, count)) in reason_counts.iter().enumerate() {
            let row = 4 + idx as u32;
            sheet
                .write_string_with_format(row, 0, excel_safe_text(reason), text_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_number_with_format(row, 1, *count as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        let reason_last = 3 + reason_counts.len() as u32;
        add_native_table(
            sheet,
            3,
            0,
            reason_last,
            1,
            "RejectionReasonTable",
            &["Rejection / Review Reason", "Count"],
            TableStyle::Medium3,
        )?;
    }

    let candidate_header_row = 3u32;
    for (col, header) in [
        "Rank",
        "Approval",
        "Primary Warning / Issue",
        "PSR Gap",
        "DSR Gap",
        "CSCV Rank Gap",
        "CSCV Metric Gap",
        "Track Gap",
        "Run PBO Margin",
        "Observed Gate Fails",
    ]
    .iter()
    .enumerate()
    {
        sheet
            .write_string_with_format(candidate_header_row, 3 + col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let rows = diagnostic_rows(model);
    let pbo_margin = run_pbo_margin(model);
    let charts_row = candidate_header_row + 2 + reason_counts.len().max(rows.len()) as u32;
    for (idx, row) in rows.iter().enumerate() {
        let r = candidate_header_row + 1 + idx as u32;
        let _ = sheet.set_row_height(r, 30.0);
        sheet
            .write_number_with_format(r, 3, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 4, display_approval_label(&row.approval_label), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                5,
                excel_safe_text(&primary_candidate_issue(row)),
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        let psr_gap = threshold_gap(row.psr, model.min_psr);
        let dsr_gap = threshold_gap(row.dsr, model.min_dsr);
        let cscv_rank_gap = threshold_gap(row.cscv_oos_rank_frac_p50, model.min_cscv_oos_rank_frac);
        let cscv_metric_gap = threshold_gap(row.cscv_oos_metric_p50, model.min_cscv_oos_metric);
        let track_gap = track_record_gap(row);
        write_optional_number(sheet, r, 6, psr_gap, pct_fmt)?;
        write_optional_number(sheet, r, 7, dsr_gap, pct_fmt)?;
        write_optional_number(sheet, r, 8, cscv_rank_gap, pct_fmt)?;
        write_optional_number(sheet, r, 9, cscv_metric_gap, num_fmt)?;
        write_optional_number(sheet, r, 10, track_gap, num_fmt)?;
        write_optional_number(sheet, r, 11, pbo_margin, pct_fmt)?;
        let failed_gate_count = observed_gate_failure_count(&[
            psr_gap,
            dsr_gap,
            cscv_rank_gap,
            cscv_metric_gap,
            track_gap,
            pbo_margin,
        ]);
        sheet
            .write_number_with_format(r, 12, failed_gate_count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !rows.is_empty() {
        let row_min = candidate_header_row + 1;
        let row_max = candidate_header_row + rows.len() as u32;
        add_native_table(
            sheet,
            candidate_header_row,
            3,
            row_max,
            12,
            "CandidateGateDistanceTable",
            &[
                "Rank",
                "Approval",
                "Primary Warning / Issue",
                "PSR Gap",
                "DSR Gap",
                "CSCV Rank Gap",
                "CSCV Metric Gap",
                "Track Gap",
                "Run PBO Margin",
                "Observed Gate Fails",
            ],
            TableStyle::Medium9,
        )?;

        apply_zero_centered_formatting(sheet, row_min, 6, row_max, 11)?;
        let no_failures = ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::EqualTo(0))
            .set_format(
                Format::new()
                    .set_font_color("166534")
                    .set_background_color("DCFCE7"),
            );
        let has_failures = ConditionalFormatCell::new()
            .set_rule(ConditionalFormatCellRule::GreaterThan(0))
            .set_format(
                Format::new()
                    .set_font_color("991B1B")
                    .set_background_color("FEE2E2"),
            );
        for rule in [&no_failures, &has_failures] {
            sheet
                .add_conditional_format(row_min, 12, row_max, 12, rule)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        let mut gap_chart = Chart::new(ChartType::Column);
        let mut gap_series_count = 0usize;
        for (name, col, available) in [
            (
                "PSR Gap",
                6u16,
                rows.iter()
                    .all(|row| threshold_gap(row.psr, model.min_psr).is_some()),
            ),
            (
                "DSR Gap",
                7u16,
                rows.iter()
                    .all(|row| threshold_gap(row.dsr, model.min_dsr).is_some()),
            ),
            (
                "CSCV Rank Gap",
                8u16,
                rows.iter().all(|row| {
                    threshold_gap(row.cscv_oos_rank_frac_p50, model.min_cscv_oos_rank_frac)
                        .is_some()
                }),
            ),
            ("PBO Margin", 11u16, pbo_margin.is_some()),
        ] {
            if available {
                gap_series_count += 1;
                gap_chart
                    .add_series()
                    .set_name(name)
                    .set_categories(("Rejection Diagnostics", row_min, 3, row_max, 3))
                    .set_values(("Rejection Diagnostics", row_min, col, row_max, col));
            }
        }
        if gap_series_count > 0 {
            gap_chart
                .title()
                .set_name("Probability and rank-fraction gate margins by candidate");
            gap_chart.x_axis().set_name("Rank");
            gap_chart.y_axis().set_name("Margin").set_num_format("0.0%");
            sheet
                .insert_chart_with_offset(charts_row, 5, &gap_chart, 8, 8)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    if !reason_counts.is_empty() {
        let reason_last = 3 + reason_counts.len() as u32;
        let mut reason_chart = Chart::new(ChartType::Bar);
        reason_chart
            .add_series()
            .set_categories(("Rejection Diagnostics", 4, 0, reason_last, 0))
            .set_values(("Rejection Diagnostics", 4, 1, reason_last, 1));
        reason_chart
            .title()
            .set_name("Rejection and review reason counts");
        reason_chart.legend().set_hidden();
        sheet
            .insert_chart_with_offset(charts_row, 0, &reason_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

fn build_report_guide(
    workbook: &mut Workbook,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Report Guide")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0x64748B));
    apply_shareable_sheet_layout(sheet, "VectorGrid Report Guide");
    let _ = sheet.set_freeze_panes(3, 0);
    let wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);
    for (col, width) in [
        (0u16, 28.0),
        (1u16, 30.0),
        (2u16, 40.0),
        (3u16, 58.0),
        (4u16, 56.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }

    sheet
        .merge_range(0, 0, 0, 4, "Report Guide", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            4,
            "Quick reference for interpreting the workbook without changing the underlying validation artifact.",
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let headers = [
        "Section",
        "Metric",
        "Direction",
        "Status / Gate",
        "Where to inspect",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let rows = [
        (
            "Research Readiness",
            "Promotion and realism evidence",
            "Fail-closed; declared evidence is better than unknown",
            "Separates statistical approval from promotion eligibility and deployment-realism declarations",
            "Readiness overview, candidate funnel, realism table, and promotion gates",
        ),
        (
            "Execution Provenance",
            "Recorded route vs expected policy",
            "Recorded runtime evidence is authoritative",
            "Separates artifact-recorded execution from request-time predictions and inferred auxiliary work",
            "Recorded execution, expected policy, and validation provenance tables",
        ),
        (
            "Candidate Comparison",
            "Curated finalist comparison",
            "Use multiple evidence dimensions",
            "Compact decision view; Audit Data remains the complete raw evidence table",
            "Candidate comparison table and decision charts",
        ),
        (
            "Dashboard",
            "Run Verdict",
            "Pass/approved is better",
            "Run-level verdict from validation result",
            "Dashboard top block",
        ),
        (
            "Validation Features",
            "Feature coverage",
            "Pass/completed is better",
            "Inventory of validation modules, derived diagnostics, skipped modules, and route evidence",
            "Validation Features table and status chart",
        ),
        (
            "Robustness Matrix",
            "PSR / DSR",
            "Higher is better",
            "Compared to Min PSR / Min DSR when configured",
            "Robustness Matrix scorecard",
        ),
        (
            "Robustness Matrix",
            "CSCV OOS rank / metric",
            "Higher is better",
            "Compared to CSCV gates when configured",
            "Robustness Matrix scorecard and peer table",
        ),
        (
            "Robustness Matrix",
            "PBO",
            "Lower is better",
            "Run PBO must be <= Max PBO when configured",
            "Robustness Matrix and Rejection Diagnostics",
        ),
        (
            "Rejection Diagnostics",
            "Gate gaps",
            "Positive is better",
            "Observed minus minimum gate; PBO margin is maximum minus observed",
            "Candidate Threshold Distances table",
        ),
        (
            "Decision Ledger",
            "Candidate decision map",
            "Pass/completed is better",
            "Heatmap plus row-level evidence for each candidate/module decision",
            "Decision Ledger heatmap, status chart, and detail table",
        ),
        (
            "Downside Stress",
            "Worst drawdown and stress evidence",
            "Lower drawdown and less negative PnL are better",
            "Compares in-sample, WF, bootstrap, regime, friction, and perturbation stress",
            "Downside Stress table and charts",
        ),
        (
            "Distributions",
            "Robust score by rank",
            "Higher is better",
            "Ranking diagnostic, not a standalone approval gate",
            "Distributions charts",
        ),
        (
            "Regime Audit",
            "Bucket Sharpe / MaxDD",
            "Higher Sharpe, lower MaxDD",
            "Describes stability across volatility/trend/range slices",
            "Regime Audit table",
        ),
        (
            "Candidate sheets",
            "Warnings / Fail Reasons",
            "Empty is better",
            "Raw candidate-level validation verdict detail",
            "Approved Candidates, Shortlist, Audit Data",
        ),
    ];
    for (idx, row) in rows.iter().enumerate() {
        let r = 4 + idx as u32;
        let _ = sheet.set_row_height(r, 42.0);
        for (col, value) in [row.0, row.1, row.2, row.3, row.4].iter().enumerate() {
            sheet
                .write_string_with_format(r, col as u16, *value, &wrap_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }
    add_native_table(
        sheet,
        3,
        0,
        3 + rows.len() as u32,
        4,
        "ReportGuideTable",
        &headers,
        TableStyle::Medium7,
    )?;

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
    dashboard.set_first_tab(true);
    dashboard.set_screen_gridlines(false);
    dashboard.set_zoom(115);
    dashboard.set_tab_color(Color::RGB(0x0F172A));
    apply_shareable_sheet_layout(dashboard, "VectorGrid Validation Dashboard");
    let _ = dashboard.set_freeze_panes(2, 0);
    let _ = dashboard.set_column_width(0, 24);
    let _ = dashboard.set_column_width(1, 34);
    let _ = dashboard.set_column_width(2, 22);
    let _ = dashboard.set_column_width(3, 34);
    let _ = dashboard.set_column_width(4, 24);
    let _ = dashboard.set_column_width(5, 24);
    let _ = dashboard.set_column_width(6, 36);
    let _ = dashboard.set_column_width(7, 32);
    let dashboard_wrap_fmt = text_fmt.clone().set_text_wrap().set_align(FormatAlign::Top);

    dashboard
        .merge_range(0, 0, 0, 7, "Validation XLSX Report", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let research_notice_fmt = review_fmt
        .clone()
        .set_text_wrap()
        .set_align(FormatAlign::Center);
    let _ = dashboard.set_row_height(1, 34.0);
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
    } else {
        dashboard
            .merge_range(
                1,
                0,
                1,
                7,
                "Research evidence only — statistical approval is not deployment authorization. Review the Research Readiness sheet before any promotion decision.",
                &research_notice_fmt,
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
    let residency_text = execution_residency_readout(model);
    let backend_reason_label = if model.actual_fallback_summary.trim() != "-" {
        "Fallback / Downgrade"
    } else {
        "Route Note"
    };
    for (label, value) in [
        ("Timestamp", model.ts.as_str()),
        ("Strategy", model.strategy_kind.as_str()),
        ("Dataset", model.data_id.as_str()),
        ("Runtime", runtime_text.as_str()),
        (
            "Acceleration Status",
            if model.validation_acceleration_status.is_empty() {
                "Validation acceleration status unavailable."
            } else {
                model.validation_acceleration_status.as_str()
            },
        ),
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
            "Requested Backend",
            if model.requested_backend.is_empty() {
                "-"
            } else {
                model.requested_backend.as_str()
            },
        ),
        ("Residency", residency_text.as_str()),
        (
            "Route",
            if model.backend_route.is_empty() {
                "-"
            } else {
                model.backend_route.as_str()
            },
        ),
        (
            backend_reason_label,
            if model.backend_fallback_reason.is_empty() {
                "-"
            } else {
                model.backend_fallback_reason.as_str()
            },
        ),
    ] {
        let row_height = if value.chars().count() > 70 {
            42.0
        } else if value.chars().count() > 42 {
            30.0
        } else {
            20.0
        };
        let _ = dashboard.set_row_height(meta_row, row_height);
        dashboard
            .write_string_with_format(meta_row, 0, label, label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .merge_range(
                meta_row,
                1,
                meta_row,
                2,
                &excel_safe_text(value),
                &dashboard_wrap_fmt,
            )
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
        let _ = dashboard.set_row_height(6, 34.0);
        let _ = dashboard.set_row_height(7, 34.0);
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
                &dashboard_wrap_fmt,
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
                &dashboard_wrap_fmt,
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
        let cscv_status = if cscv.skipped_reason.is_empty() {
            "completed".to_string()
        } else {
            format!("skipped: {}", cscv.skipped_reason)
        };
        dashboard
            .write_string_with_format(8, 6, &cscv_status, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(
                3,
                6,
                disabled_or_omitted_status(model, "cscv_pbo"),
                text_fmt,
            )
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
        let spa_status = if !spa.skipped_reason.is_empty() {
            format!("skipped: {}", spa.skipped_reason)
        } else if spa.passed == Some(true) {
            "pass".to_string()
        } else if spa.passed == Some(false) {
            "fail".to_string()
        } else {
            "not run".to_string()
        };
        dashboard
            .write_string_with_format(13, 6, &spa_status, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(10, 6, disabled_or_omitted_status(model, "spa"), text_fmt)
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
        write_optional_number(dashboard, 20, 6, reopt.forward_max_dd_worst, pct_fmt)?;
        dashboard
            .write_string_with_format(21, 5, "Fwd PnL+", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 21, 6, reopt.forward_positive_pnl_frac, pct_fmt)?;
        dashboard
            .write_string_with_format(22, 5, "WFE Sharpe P50", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 22, 6, reopt.wfe_sharpe_median, num_fmt)?;
        dashboard
            .write_string_with_format(23, 5, "WFE Sharpe Worst", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 23, 6, reopt.wfe_sharpe_worst, num_fmt)?;
        dashboard
            .write_string_with_format(24, 5, "WFE Sharpe Std", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 24, 6, reopt.wfe_sharpe_std, num_fmt)?;
        dashboard
            .write_string_with_format(25, 5, "WFE PnL P50", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 25, 6, reopt.wfe_pnl_median, num_fmt)?;
        dashboard
            .write_string_with_format(26, 5, "WFE PnL Worst", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 26, 6, reopt.wfe_pnl_worst, num_fmt)?;
    } else {
        dashboard
            .write_string_with_format(
                15,
                6,
                disabled_or_omitted_status(model, "reopt_walk_forward"),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(28, 5, "Cross-Ticker", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(cross) = model.cross_ticker.as_ref() {
        dashboard
            .write_string_with_format(29, 5, "Datasets", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(29, 6, cross.dataset_count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(30, 5, "Mode", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(30, 6, display_cross_mode_label(&cross.mode), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(31, 5, "Labels", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                31,
                6,
                if cross.dataset_labels.is_empty() {
                    "-"
                } else {
                    cross.dataset_labels.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(32, 5, "Shortlist Count", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(32, 6, cross.shortlist_count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(33, 5, "Semantics", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(33, 6, display_cross_semantics_label(&cross.mode), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(
                29,
                6,
                disabled_or_omitted_status(model, "cross_ticker"),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let top_warning = run_verdict
        .and_then(|verdict| {
            if !verdict.fail_reasons.is_empty() {
                Some(verdict.fail_reasons.as_str())
            } else if !verdict.warning_flags.is_empty() {
                Some(verdict.warning_flags.as_str())
            } else {
                None
            }
        })
        .unwrap_or("-");

    dashboard
        .write_string_with_format(34, 5, "Execution Notes", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(35, 5, "Route Fallback", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            35,
            6,
            excel_safe_text(&model.actual_fallback_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(36, 5, "Route Note", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            36,
            6,
            excel_safe_text(&model.actual_route_note_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(37, 5, "Aux/Mixed Tails (inferred)", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            37,
            6,
            excel_safe_text(&model.actual_aux_tail_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(38, 5, "Skipped Modules", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            38,
            6,
            excel_safe_text(&model.actual_skipped_module_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(39, 5, "Disabled Modules", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            39,
            6,
            excel_safe_text(&model.actual_disabled_module_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(40, 5, "Cross Omitted Modules", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            40,
            6,
            excel_safe_text(&model.actual_cross_omitted_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(41, 5, "Top Warning", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(41, 6, excel_safe_text(top_warning), text_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(42, 5, "Expected Execution", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(43, 5, "Expected Route", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            43,
            6,
            excel_safe_text(&model.expected_route_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(44, 5, "Expected Cross", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            44,
            6,
            excel_safe_text(&model.expected_cross_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(45, 5, "Expected Modules", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            45,
            6,
            excel_safe_text(&model.expected_module_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(46, 5, "Expected Categories", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            46,
            6,
            excel_safe_text(&model.expected_category_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(47, 5, "Expected Fallback", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            47,
            6,
            excel_safe_text(&model.expected_fallback_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(48, 5, "Expected Route Notes", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            48,
            6,
            excel_safe_text(&model.expected_route_note_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(49, 5, "Expected Disabled", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            49,
            6,
            excel_safe_text(&model.expected_disabled_module_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(50, 5, "Expected Cross Omitted", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            50,
            6,
            excel_safe_text(&model.expected_cross_omitted_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(51, 5, "Expected Notes", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(
            51,
            6,
            excel_safe_text(&model.expected_warning_summary),
            text_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    for (row, value) in [
        (35u32, model.actual_fallback_summary.as_str()),
        (36, model.actual_route_note_summary.as_str()),
        (37, model.actual_aux_tail_summary.as_str()),
        (38, model.actual_skipped_module_summary.as_str()),
        (39, model.actual_disabled_module_summary.as_str()),
        (40, model.actual_cross_omitted_summary.as_str()),
        (41, top_warning),
        (43, model.expected_route_summary.as_str()),
        (44, model.expected_cross_summary.as_str()),
        (45, model.expected_module_summary.as_str()),
        (46, model.expected_category_summary.as_str()),
        (47, model.expected_fallback_summary.as_str()),
        (48, model.expected_route_note_summary.as_str()),
        (49, model.expected_disabled_module_summary.as_str()),
        (50, model.expected_cross_omitted_summary.as_str()),
        (51, model.expected_warning_summary.as_str()),
    ] {
        let row_height = wrapped_row_height(value, 82, 20.0, 120.0);
        let _ = dashboard.set_row_height(row, row_height);
        dashboard
            .merge_range(row, 6, row, 7, &excel_safe_text(value), &dashboard_wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(32, 3, "Gate Config", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (offset, label, value, is_pct) in [
        (1u32, "Min PSR", model.min_psr, true),
        (2u32, "Min DSR", model.min_dsr, true),
        (3u32, "Min CSCV Rank", model.min_cscv_oos_rank_frac, true),
        (4u32, "Min CSCV Metric", model.min_cscv_oos_metric, false),
        (5u32, "Max PBO", model.max_pbo, true),
    ] {
        let row = 32 + offset;
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

    let approved_link = internal_sheet_link("Approved Candidates", "A1");
    let shortlist_link = internal_sheet_link("Shortlist", "A1");
    let distributions_link = internal_sheet_link("Distributions", "A1");
    let audit_link = internal_sheet_link("Audit Data", "A1");
    let features_link = internal_sheet_link("Validation Features", "A1");
    let robustness_link = internal_sheet_link("Robustness Matrix", "A1");
    let diagnostics_link = internal_sheet_link("Rejection Diagnostics", "A1");
    let guide_link = internal_sheet_link("Report Guide", "A1");
    let decision_link = internal_sheet_link("Decision Ledger", "A1");
    let downside_link = internal_sheet_link("Downside Stress", "A1");
    let regime_link = internal_sheet_link("Regime Audit", "A1");
    let comparison_link = internal_sheet_link("Candidate Comparison", "A1");
    let readiness_link = internal_sheet_link("Research Readiness", "A1");
    let provenance_link = internal_sheet_link("Execution Provenance", "A1");
    dashboard
        .write_string_with_format(13, 0, "Sheet Links", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dashboard.write_url_with_text(13, 1, approved_link.as_str(), "Approved");
    let _ = dashboard.write_url_with_text(13, 2, shortlist_link.as_str(), "Shortlist");
    let _ = dashboard.write_url_with_text(13, 3, features_link.as_str(), "Features");
    let _ = dashboard.write_url_with_text(13, 4, decision_link.as_str(), "Decisions");
    dashboard
        .write_string_with_format(14, 0, "Analysis Links", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dashboard.write_url_with_text(14, 1, downside_link.as_str(), "Downside");
    let _ = dashboard.write_url_with_text(14, 2, robustness_link.as_str(), "Robustness");
    let _ = dashboard.write_url_with_text(14, 3, diagnostics_link.as_str(), "Diagnostics");
    let _ = dashboard.write_url_with_text(14, 4, audit_link.as_str(), "Audit Data");
    dashboard
        .write_string_with_format(91, 0, "Reference Links", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dashboard.write_url_with_text(91, 1, distributions_link.as_str(), "Distributions");
    let _ = dashboard.write_url_with_text(91, 2, regime_link.as_str(), "Regime");
    let _ = dashboard.write_url_with_text(91, 3, guide_link.as_str(), "Guide");
    let _ = dashboard.write_url_with_text(91, 4, comparison_link.as_str(), "Compare");
    let _ = dashboard.write_url_with_text(91, 5, readiness_link.as_str(), "Readiness");
    let _ = dashboard.write_url_with_text(91, 6, provenance_link.as_str(), "Provenance");

    dashboard
        .write_string_with_format(15, 0, "Top Candidate Snapshot", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let top_candidate = model
        .approved_rows
        .first()
        .or_else(|| model.shortlist_rows.first())
        .or_else(|| model.all_rows.first());
    if let Some(row) = top_candidate {
        let plateau_text = plateau_summary_label(row);
        for (offset, (label, value)) in [
            ("Approval", display_approval_label(&row.approval_label)),
            ("Reason", row.approval_reason.as_str()),
            ("Profile", row.profile_kind.as_str()),
            ("Params", row.params_summary.as_str()),
            ("Plateau", plateau_text.as_str()),
            (
                "Plateau Detail",
                if row.plateau_source.is_empty() {
                    "-"
                } else if row.plateau_source.eq_ignore_ascii_case("top") {
                    "top slice neighborhood"
                } else {
                    "full optimization surface"
                },
            ),
        ]
        .iter()
        .enumerate()
        {
            let base_row = 16 + offset as u32;
            let row_height = if value.chars().count() > 70 {
                46.0
            } else if value.chars().count() > 38 {
                32.0
            } else {
                20.0
            };
            let _ = dashboard.set_row_height(base_row, row_height);
            dashboard
                .write_string_with_format(base_row, 0, *label, label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            dashboard
                .merge_range(
                    base_row,
                    1,
                    base_row,
                    2,
                    &excel_safe_text(value),
                    &dashboard_wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
        }
        for (offset, (label, value, pct_metric)) in [
            ("Robust", Some(row.robust_score), false),
            ("PSR", row.psr, true),
            ("DSR", row.dsr, true),
            ("WF Sharpe", row.wf_sharpe_median, false),
            ("WF PnL Eff", row.wf_pnl_efficiency, false),
            ("WF Sharpe Eff", row.wf_sharpe_efficiency, false),
            ("Cross Score", row.cross_score, false),
            ("Bootstrap Prob+", row.bootstrap_prob_pnl_positive, true),
            ("Bootstrap P05", row.bootstrap_pnl_p05, false),
            ("Bootstrap DD95", row.bootstrap_dd_p95, true),
            ("Stat P05", row.bootstrap_stationary_pnl_p05, false),
            ("Stat DD95", row.bootstrap_stationary_dd_p95, true),
            ("Plateau Score", row.plateau_score, false),
            ("Plateau Sharpe P50", row.plateau_sharpe_median, false),
            ("Plateau DD Worst", row.plateau_max_dd_worst, true),
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
        .write_string_with_format(52, 5, "Ranking Stability", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(stability) = model.ranking_stability.as_ref() {
        for (offset, label, value) in [
            (
                1u32,
                "Finalist Top-K",
                format!("{}", stability.finalist_k.max(1)),
            ),
            (
                2u32,
                "WF overlap/drift",
                match (stability.wf_overlap, stability.wf_mean_drift) {
                    (Some(overlap), Some(drift)) => {
                        format!("{overlap}/{} | {drift:.2}", stability.finalist_k.max(1))
                    }
                    _ => "not run".to_string(),
                },
            ),
            (
                3u32,
                "CSCV overlap/drift",
                match (stability.cscv_overlap, stability.cscv_mean_drift) {
                    (Some(overlap), Some(drift)) => {
                        format!("{overlap}/{} | {drift:.2}", stability.finalist_k.max(1))
                    }
                    _ => "not run".to_string(),
                },
            ),
            (
                4u32,
                "Cross overlap/drift",
                match (stability.cross_overlap, stability.cross_mean_drift) {
                    (Some(overlap), Some(drift)) => {
                        format!("{overlap}/{} | {drift:.2}", stability.finalist_k.max(1))
                    }
                    _ => "not run".to_string(),
                },
            ),
            (
                5u32,
                "Stable finalists",
                format!(
                    "{}/{}",
                    stability.stable_finalist_count,
                    stability.finalist_k.max(1)
                ),
            ),
        ] {
            let row = 52 + offset;
            dashboard
                .write_string_with_format(row, 5, label, label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            dashboard
                .write_string_with_format(row, 6, &value, text_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    } else {
        dashboard
            .write_string_with_format(53, 6, "not available", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(59, 5, "WF Efficiency", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let efficiency_rows = if model.approved_rows.is_empty() {
        &model.shortlist_rows
    } else {
        &model.approved_rows
    };
    let median_wf_pnl_eff =
        median_option_f64(efficiency_rows.iter().map(|row| row.wf_pnl_efficiency));
    let median_wf_sharpe_eff =
        median_option_f64(efficiency_rows.iter().map(|row| row.wf_sharpe_efficiency));
    for (offset, label, value) in [
        (
            1u32,
            "Top PnL Eff",
            top_candidate.and_then(|row| row.wf_pnl_efficiency),
        ),
        (
            2u32,
            "Top Sharpe Eff",
            top_candidate.and_then(|row| row.wf_sharpe_efficiency),
        ),
        (3u32, "Median PnL Eff", median_wf_pnl_eff),
        (4u32, "Median Sharpe Eff", median_wf_sharpe_eff),
    ] {
        let row = 59 + offset;
        dashboard
            .write_string_with_format(row, 5, label, label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, row, 6, value, num_fmt)?;
    }
    dashboard
        .write_string_with_format(64, 5, "Policy", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .merge_range(
            64,
            6,
            64,
            7,
            &excel_safe_text(&model.wfe_policy_summary),
            &dashboard_wrap_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dashboard.set_row_height(64, 34.0);

    dashboard
        .write_string_with_format(65, 5, "Regime Robustness", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(row) = top_candidate {
        dashboard
            .write_string_with_format(66, 5, "Worst Bucket", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                66,
                6,
                excel_safe_text(if row.regime_worst_bucket.is_empty() {
                    "not run"
                } else {
                    row.regime_worst_bucket.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(67, 5, "Worst Sharpe", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 67, 6, row.regime_worst_sharpe, num_fmt)?;
        dashboard
            .write_string_with_format(68, 5, "Worst MaxDD", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 68, 6, row.regime_worst_max_dd, pct_fmt)?;
        dashboard
            .write_string_with_format(69, 5, "Stable Buckets", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(dashboard, 69, 6, row.regime_stable_bucket_count, int_fmt)?;
        dashboard
            .write_string_with_format(70, 5, "Stable Frac", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 70, 6, row.regime_stable_bucket_frac, pct_fmt)?;
        dashboard
            .write_string_with_format(71, 5, "Flag", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                71,
                6,
                display_regime_flag(&row.regime_fragility_flag),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(72, 5, "Support", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                72,
                6,
                display_regime_support(&row.regime_support_label),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(73, 5, "Slice Audit", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .merge_range(
                73,
                6,
                73,
                7,
                "see Audit Data -> Regime Slice Audit",
                &dashboard_wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        dashboard
            .write_string_with_format(66, 6, "not available", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(74, 5, "Bootstrap Compare", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(row) = top_candidate {
        dashboard
            .write_string_with_format(75, 5, "Mode", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(75, 6, bootstrap_mode_label(row), text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(76, 5, "Stat Block", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(
            dashboard,
            76,
            6,
            row.bootstrap_stationary_mean_block_len,
            int_fmt,
        )?;
        dashboard
            .write_string_with_format(77, 5, "IID P05", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 77, 6, row.bootstrap_pnl_p05, num_fmt)?;
        dashboard
            .write_string_with_format(78, 5, "IID DD95", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 78, 6, row.bootstrap_dd_p95, pct_fmt)?;
        dashboard
            .write_string_with_format(79, 5, "Stat P05", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 79, 6, row.bootstrap_stationary_pnl_p05, num_fmt)?;
        dashboard
            .write_string_with_format(80, 5, "Stat DD95", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 80, 6, row.bootstrap_stationary_dd_p95, pct_fmt)?;
        dashboard
            .write_string_with_format(81, 5, "Note", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .merge_range(
                81,
                6,
                81,
                7,
                &excel_safe_text(&model.bootstrap_mode_summary),
                &dashboard_wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        let _ = dashboard.set_row_height(81, 30.0);
    } else {
        dashboard
            .write_string_with_format(75, 6, "not available", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(82, 5, "Friction Stability", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(row) = top_candidate {
        dashboard
            .write_string_with_format(83, 5, "Worst Scenario", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                83,
                6,
                excel_safe_text(if row.friction_worst_scenario.is_empty() {
                    "not run"
                } else {
                    row.friction_worst_scenario.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(84, 5, "Worst-Scenario Sharpe", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 84, 6, row.friction_worst_sharpe, num_fmt)?;
        dashboard
            .write_string_with_format(85, 5, "Worst-Scenario MaxDD", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 85, 6, row.friction_worst_max_dd, pct_fmt)?;
        dashboard
            .write_string_with_format(86, 5, "Pass Frac", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, 86, 6, row.friction_pass_frac, pct_fmt)?;
        dashboard
            .write_string_with_format(87, 5, "Max Rank Drift", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(dashboard, 87, 6, row.friction_max_rank_drift, int_fmt)?;
        dashboard
            .write_string_with_format(88, 5, "Flag", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                88,
                6,
                excel_safe_text(if row.friction_flag.is_empty() {
                    "not run"
                } else {
                    row.friction_flag.as_str()
                }),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(89, 5, "Rule", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .merge_range(
                89,
                6,
                89,
                7,
                "absolute pass: pnl >= 0 and sharpe >= 0",
                &dashboard_wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(90, 5, "Note", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .merge_range(
                90,
                6,
                90,
                7,
                &excel_safe_text(&model.friction_policy_summary),
                &dashboard_wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        let _ = dashboard.set_row_height(90, 34.0);
    } else {
        dashboard
            .write_string_with_format(83, 6, "not available", text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let top_rows_title = 94u32;
    let top_rows_header = top_rows_title + 1;
    let top_rows_start = top_rows_header + 1;
    dashboard
        .write_string_with_format(top_rows_title, 0, "Top Visible Rows", label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (col, header) in [
        "Rank", "Approval", "Profile", "Params", "Robust", "PSR", "DSR", "Reason",
    ]
    .iter()
    .enumerate()
    {
        dashboard
            .write_string_with_format(top_rows_header, col as u16, *header, label_fmt)
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
        let r = top_rows_start + idx as u32;
        let row_height = wrapped_row_height(&row.params_summary, 34, 34.0, 90.0)
            .max(wrapped_row_height(&row.approval_reason, 32, 34.0, 90.0));
        let _ = dashboard.set_row_height(r, row_height);
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
            .write_string_with_format(
                r,
                1,
                display_approval_label(&row.approval_label),
                status_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                r,
                2,
                excel_safe_text(&row.profile_kind),
                &dashboard_wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(
                r,
                3,
                excel_safe_text(&row.params_summary),
                &dashboard_wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(r, 4, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dashboard, r, 5, row.psr, pct_fmt)?;
        write_optional_number(dashboard, r, 6, row.dsr, pct_fmt)?;
        dashboard
            .write_string_with_format(
                r,
                7,
                excel_safe_text(&row.approval_reason),
                &dashboard_wrap_fmt,
            )
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
        rr_chart.x_axis().set_name("MaxDD").set_num_format("0.0%");
        rr_chart.y_axis().set_name("Sharpe");
        rr_chart.legend().set_hidden();
        dashboard
            .insert_chart_with_offset(20, 9, &rr_chart, 8, 8)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    Ok(())
}

fn display_identifier(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "Not recorded in artifact".to_string()
    } else {
        trimmed.replace('_', " ")
    }
}

fn terminal_route_residency(route: &str) -> Option<(&'static str, &'static str)> {
    match route
        .rsplit("->")
        .next()
        .unwrap_or(route)
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "gpukernel" => Some(("GpuKernel", "gpu_kernel")),
        "gpuhybridkernel" => Some(("GpuHybridKernel", "gpu_hybrid_kernel")),
        "cpuonly" | "cpu" => Some(("CpuOnly", "cpu")),
        _ => None,
    }
}

fn explicit_residency_tier(value: &str) -> Option<&'static str> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.contains("hybrid") {
        Some("gpu_hybrid_kernel")
    } else if normalized == "gpu_kernel" || normalized.contains("gpukernel") {
        Some("gpu_kernel")
    } else if normalized == "cpu" || normalized == "cpu_only" || normalized == "cpuonly" {
        Some("cpu")
    } else {
        None
    }
}

fn execution_residency_readout(model: &ValidationExportModel) -> String {
    let pack = model.validation_evidence_pack.as_ref();
    let recorded = if model.validation_residency_tier.trim().is_empty() {
        pack.map(|value| value.residency_tier.as_str())
            .unwrap_or("")
    } else {
        model.validation_residency_tier.as_str()
    };
    if recorded.trim().is_empty() {
        return "Not recorded in artifact".to_string();
    }
    let effective_route = if model.backend_route.trim().is_empty() {
        pack.map(|value| value.route_label.as_str()).unwrap_or("")
    } else {
        model.backend_route.as_str()
    };
    if let (Some(recorded_tier), Some((terminal_route, terminal_tier))) = (
        explicit_residency_tier(recorded),
        terminal_route_residency(effective_route),
    ) {
        if recorded_tier != terminal_tier {
            return format!(
                "{} (artifact; terminal route {} implies {}; provenance mismatch)",
                display_identifier(recorded),
                terminal_route,
                terminal_tier
            );
        }
    }
    display_identifier(recorded)
}

fn realism_interpretation(dimension: &str, status: &str) -> &'static str {
    match (dimension, status.trim()) {
        ("Price adjustment", "adjusted") => {
            "Artifact declares adjusted price data; declaration is not independent verification."
        }
        ("Price adjustment", "unadjusted") => {
            "Artifact declares unadjusted prices; corporate-action effects require review."
        }
        (_, "documented") => {
            "Evidence is declared in the artifact; this is not independent verification."
        }
        (_, "not_applicable") => "Artifact explicitly marks this realism dimension not applicable.",
        _ => "Unknown or absent evidence blocks promotion beyond research use.",
    }
}

fn promotion_evidence_is_eligible_for_review(pack: &ExportValidationEvidencePack) -> bool {
    let price_adjustment_is_declared = |value: &str| {
        let value = value.trim();
        value.eq_ignore_ascii_case("adjusted")
            || value.eq_ignore_ascii_case("unadjusted")
            || value.eq_ignore_ascii_case("not_applicable")
    };
    let realism_status_is_declared = |value: &str| {
        let value = value.trim();
        value.eq_ignore_ascii_case("documented") || value.eq_ignore_ascii_case("not_applicable")
    };
    pack.schema_version.trim() == ta_app_core::VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION
        && pack.version == Some(u64::from(ta_app_core::VALIDATION_EVIDENCE_PACK_VERSION))
        && pack.status.trim() == "complete"
        && pack.verdict_passed == Some(true)
        && pack.verdict_approved == Some(true)
        && pack.promotion_eligible == Some(true)
        && pack.promotion_state.trim() == "eligible_for_review"
        && !pack.promotion_reason.trim().is_empty()
        && pack.realism_complete == Some(true)
        && pack.realism_missing_or_unknown.trim().is_empty()
        && price_adjustment_is_declared(&pack.realism_price_adjustment)
        && [
            pack.realism_corporate_actions.as_str(),
            pack.realism_survivorship_bias.as_str(),
            pack.realism_liquidity_capacity.as_str(),
            pack.realism_market_impact.as_str(),
            pack.realism_borrow_availability.as_str(),
        ]
        .into_iter()
        .all(realism_status_is_declared)
        && pack.promotion_gates_failed.trim().is_empty()
}

fn evidence_pack_candidate_funnel(
    pack: &ExportValidationEvidencePack,
    allow_empty: bool,
) -> Option<[usize; 5]> {
    let counts = [
        pack.requested_candidates?,
        pack.evaluated_candidates?,
        pack.retained_candidates?,
        pack.survived_hard_gates?,
        pack.approved_candidates?,
    ];
    if !allow_empty && counts[0] == 0 {
        return None;
    }
    counts
        .windows(2)
        .all(|pair| pair[0] >= pair[1])
        .then_some(counts)
}

fn observed_hard_gate_survivors(model: &ValidationExportModel) -> usize {
    let has_candidate_gate_evidence = model
        .all_rows
        .iter()
        .any(|row| row.hard_gate_passed.is_some());
    if has_candidate_gate_evidence {
        return model
            .all_rows
            .iter()
            .filter(|row| row.hard_gate_passed == Some(true))
            .count();
    }
    model
        .run_verdict
        .as_ref()
        .map(|value| value.candidates_survived_hard_gates)
        .filter(|value| *value > 0)
        .unwrap_or(model.approved_rows.len())
        .min(model.all_rows.len())
}

fn build_candidate_comparison_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
    compat_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Candidate Comparison")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(105);
    sheet.set_tab_color(Color::RGB(0x2563EB));
    sheet.set_screen_gridlines(false);
    apply_shareable_sheet_layout(sheet, "VectorGrid Candidate Comparison");
    let _ = sheet.set_freeze_panes(4, 6);
    let wrap_fmt = Format::new()
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);
    for (col, width) in [
        (0u16, 8.0),
        (1, 16.0),
        (2, 11.0),
        (3, 9.0),
        (4, 15.0),
        (5, 42.0),
        (6, 12.0),
        (7, 10.0),
        (8, 10.0),
        (9, 12.0),
        (10, 12.0),
        (11, 12.0),
        (12, 11.0),
        (13, 12.0),
        (14, 12.0),
        (15, 12.0),
        (16, 14.0),
        (17, 13.0),
        (18, 13.0),
        (19, 11.0),
        (20, 40.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }
    sheet
        .merge_range(0, 0, 0, 20, "Candidate Comparison", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            20,
            "Curated decision view of the visible shortlist. Audit Data retains the complete 60-column evidence surface.",
            &wrap_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let headers = [
        "Rank",
        "Decision",
        "Hard Gate",
        "Pareto",
        "Profile",
        "Parameters",
        "Robust Score",
        "PSR",
        "DSR",
        "PnL",
        "Sharpe",
        "MaxDD",
        "Exposure",
        "WF Sharpe",
        "CSCV Rank",
        "Cross Score",
        "Bootstrap Prob+",
        "Regime",
        "Friction",
        "Max Drift",
        "Primary Reason",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (idx, row) in model.shortlist_rows.iter().enumerate() {
        let r = 4 + idx as u32;
        let primary_issue = primary_candidate_issue(row);
        let row_height = wrapped_row_height(&row.params_summary, 38, 34.0, 90.0)
            .max(wrapped_row_height(&primary_issue, 46, 34.0, 90.0));
        let _ = sheet.set_row_height(r, row_height);
        sheet
            .write_number_with_format(r, 0, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        let decision_fmt = match row.approval_label.as_str() {
            "approved" => approved_fmt,
            "rejected" => rejected_fmt,
            "compat_shortlist" | "compat_context" => compat_fmt,
            _ => review_fmt,
        };
        sheet
            .write_string_with_format(
                r,
                1,
                display_approval_label(&row.approval_label),
                decision_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                2,
                match row.hard_gate_passed {
                    Some(true) => "Pass",
                    Some(false) => "Fail",
                    None => "Not recorded",
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 3, if row.robust_pareto { "Yes" } else { "No" }, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 4, &row.profile_kind, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(r, 5, excel_safe_text(&row.params_summary), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 6, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 7, row.psr, pct_fmt)?;
        write_optional_number(sheet, r, 8, row.dsr, pct_fmt)?;
        sheet
            .write_number_with_format(r, 9, row.pnl, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 10, row.sharpe, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(r, 11, row.max_dd, pct_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(sheet, r, 12, row.exposure, pct_fmt)?;
        write_optional_number(sheet, r, 13, row.wf_sharpe_median, num_fmt)?;
        write_optional_number(sheet, r, 14, row.cscv_oos_rank_frac_p50, pct_fmt)?;
        write_optional_number(sheet, r, 15, row.cross_score, num_fmt)?;
        write_optional_number(sheet, r, 16, row.bootstrap_prob_pnl_positive, pct_fmt)?;
        sheet
            .write_string_with_format(
                r,
                17,
                display_regime_flag(&row.regime_fragility_flag),
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                r,
                18,
                if row.friction_flag.trim().is_empty() {
                    "Not run"
                } else {
                    row.friction_flag.as_str()
                },
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_usize(sheet, r, 19, row.max_view_rank_drift, int_fmt)?;
        sheet
            .write_string_with_format(r, 20, excel_safe_text(&primary_issue), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    if model.shortlist_rows.is_empty() {
        sheet
            .write_string_with_format(
                4,
                0,
                "No candidate rows present in this artifact.",
                text_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        return Ok(());
    }

    let row_min = 4u32;
    let row_max = 3 + model.shortlist_rows.len() as u32;
    add_native_table(
        sheet,
        3,
        0,
        row_max,
        20,
        "CandidateComparisonTable",
        &headers,
        TableStyle::Medium2,
    )?;
    let good_scale = ConditionalFormat3ColorScale::new()
        .set_minimum_color("F8696B")
        .set_midpoint_color("FFEB84")
        .set_maximum_color("63BE7B");
    let bad_scale = ConditionalFormat3ColorScale::new()
        .set_minimum_color("63BE7B")
        .set_midpoint_color("FFEB84")
        .set_maximum_color("F8696B");
    sheet
        .add_conditional_format(row_min, 6, row_max, 6, &good_scale)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .add_conditional_format(row_min, 11, row_max, 11, &bad_scale)
        .map_err(|e: XlsxError| e.to_string())?;
    for col in [9u16, 10, 13, 15] {
        apply_zero_centered_formatting(sheet, row_min, col, row_max, col)?;
    }
    let probability_bar = probability_data_bar("5A8AC6");
    for col in [7u16, 8, 12, 14, 16] {
        sheet
            .add_conditional_format(row_min, col, row_max, col, &probability_bar)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let chart_row_max = row_min + model.shortlist_rows.len().min(SUMMARY_CHART_ROW_CAP) as u32 - 1;
    let mut robust_chart = Chart::new(ChartType::Column);
    robust_chart
        .add_series()
        .set_categories(("Candidate Comparison", row_min, 0, chart_row_max, 0))
        .set_values(("Candidate Comparison", row_min, 6, chart_row_max, 6))
        .set_format(ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#2563EB")));
    robust_chart.title().set_name(&capped_chart_title(
        "Robust score by candidate rank",
        model.shortlist_rows.len(),
    ));
    robust_chart.x_axis().set_name("Rank");
    robust_chart.y_axis().set_name("Robust score");
    robust_chart.legend().set_hidden();
    robust_chart
        .set_width(520)
        .set_height(300)
        .set_alt_text("Column chart of robust score by candidate rank.");
    sheet
        .insert_chart_with_offset(3, 22, &robust_chart, 8, 8)
        .map_err(|e: XlsxError| e.to_string())?;

    let mut risk_marker = ChartMarker::new();
    risk_marker
        .set_type(ChartMarkerType::Circle)
        .set_size(6)
        .set_format(ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#0EA5E9")));
    let mut risk_chart = Chart::new(ChartType::Scatter);
    risk_chart
        .add_series()
        .set_categories(("Candidate Comparison", row_min, 11, chart_row_max, 11))
        .set_values(("Candidate Comparison", row_min, 10, chart_row_max, 10))
        .set_marker(&risk_marker);
    risk_chart.title().set_name(&capped_chart_title(
        "Sharpe vs downside",
        model.shortlist_rows.len(),
    ));
    risk_chart.x_axis().set_name("MaxDD").set_num_format("0.0%");
    risk_chart.y_axis().set_name("Sharpe");
    risk_chart.legend().set_hidden();
    risk_chart
        .set_width(520)
        .set_height(300)
        .set_alt_text("Scatter chart comparing candidate Sharpe with maximum drawdown.");
    sheet
        .insert_chart_with_offset(20, 22, &risk_chart, 8, 8)
        .map_err(|e: XlsxError| e.to_string())?;
    Ok(())
}

fn build_research_readiness_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    int_fmt: &Format,
    approved_fmt: &Format,
    review_fmt: &Format,
    rejected_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Research Readiness")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(110);
    sheet.set_tab_color(Color::RGB(0xF59E0B));
    sheet.set_screen_gridlines(false);
    apply_shareable_sheet_layout(sheet, "VectorGrid Research Readiness");
    let _ = sheet.set_freeze_panes(4, 0);
    let wrap_fmt = Format::new()
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);
    let banner_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0x92400E))
        .set_background_color(Color::RGB(0xFEF3C7))
        .set_text_wrap()
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);
    for (col, width) in [
        (0u16, 26.0),
        (1, 34.0),
        (2, 60.0),
        (4, 34.0),
        (5, 18.0),
        (6, 18.0),
        (8, 18.0),
        (9, 14.0),
    ] {
        let _ = sheet.set_column_width(col, width);
    }
    sheet
        .merge_range(
            0,
            0,
            0,
            12,
            "Research Readiness & Promotion Evidence",
            title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            12,
            "Research evidence only — statistical approval is not deployment authorization. Promotion remains fail-closed when realism evidence is unknown.",
            &banner_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = sheet.set_row_height(1, 36.0);

    let pack = model.validation_evidence_pack.as_ref();
    let verdict = model.run_verdict.as_ref();
    let pack_promotion_is_eligible = pack
        .map(promotion_evidence_is_eligible_for_review)
        .unwrap_or(false);
    let top_level_verdict_allows_promotion = verdict
        .map(|value| value.passed && value.approved)
        .unwrap_or(true);
    let promotion_is_eligible = pack_promotion_is_eligible && top_level_verdict_allows_promotion;
    let promotion_verdict_conflict =
        pack_promotion_is_eligible && !top_level_verdict_allows_promotion;
    let promotion_state = match pack {
        None => "Research only (legacy evidence pack absent)".to_string(),
        Some(_) if promotion_is_eligible => "Eligible for review".to_string(),
        Some(_) if promotion_verdict_conflict => {
            "Research only (fail-closed; top-level run verdict conflicts with evidence pack)"
                .to_string()
        }
        Some(value) if value.promotion_state.trim().eq_ignore_ascii_case("blocked") => {
            "Blocked".to_string()
        }
        Some(value)
            if value
                .promotion_state
                .trim()
                .eq_ignore_ascii_case("research_only") =>
        {
            "Research only".to_string()
        }
        Some(value) => format!(
            "Research only (fail-closed; recorded state: {})",
            display_identifier(&value.promotion_state)
        ),
    };
    let promotion_fmt = if promotion_is_eligible {
        approved_fmt
    } else if pack.is_some_and(|value| {
        value.promotion_state.trim().eq_ignore_ascii_case("blocked")
            || value.promotion_eligible == Some(true)
            || value
                .promotion_state
                .trim()
                .eq_ignore_ascii_case("eligible_for_review")
    }) {
        rejected_fmt
    } else {
        review_fmt
    };
    let statistical_status = match verdict {
        Some(value) => {
            if value.approved {
                "Approved as research evidence".to_string()
            } else if value.passed {
                "Hard gates passed; not approved".to_string()
            } else {
                "Not approved".to_string()
            }
        }
        None => match pack.map(|value| (value.verdict_passed, value.verdict_approved)) {
            Some((Some(true), Some(true))) => {
                "Approved as research evidence (evidence pack)".to_string()
            }
            Some((Some(true), Some(false))) => {
                "Hard gates passed; not approved (evidence pack)".to_string()
            }
            Some((Some(false), _)) => "Not approved (evidence pack)".to_string(),
            _ => "Not recorded in artifact".to_string(),
        },
    };
    let promotion_eligible = match pack {
        None => "Not recorded; treated as research only",
        Some(_) if promotion_is_eligible => "Yes — eligible for human review",
        Some(_) if promotion_verdict_conflict => {
            "No — fail-closed because the top-level run verdict conflicts with the evidence pack"
        }
        Some(value)
            if value.promotion_eligible == Some(true)
                || value
                    .promotion_state
                    .trim()
                    .eq_ignore_ascii_case("eligible_for_review") =>
        {
            "No — fail-closed because the recorded promotion evidence is inconsistent"
        }
        Some(_) => "No",
    };
    let overview = [
        ("Statistical validation", statistical_status),
        ("Promotion state", promotion_state),
        ("Promotion eligible", promotion_eligible.to_string()),
        (
            "Evidence pack status",
            pack.map(|value| display_identifier(&value.status))
                .unwrap_or_else(|| "Not recorded in artifact".to_string()),
        ),
        (
            "Evidence schema",
            pack.map(|value| {
                if value.schema_version.trim().is_empty() {
                    "Not recorded in artifact".to_string()
                } else {
                    format!(
                        "{} / v{}",
                        value.schema_version,
                        value
                            .version
                            .map(|version| version.to_string())
                            .unwrap_or_else(|| "?".to_string())
                    )
                }
            })
            .unwrap_or_else(|| "Legacy artifact".to_string()),
        ),
        (
            "Promotion reason",
            pack.map(|value| display_identifier(&value.promotion_reason))
                .unwrap_or_else(|| {
                    "Promotion evidence is absent; no deployment readiness may be inferred."
                        .to_string()
                }),
        ),
        (
            "Evidence status reason",
            pack.map(|value| display_identifier(&value.status_reason))
                .unwrap_or_else(|| "Not recorded in artifact".to_string()),
        ),
    ];
    for (col, header) in ["Readiness Field", "Recorded Value", "Interpretation"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (idx, (field, value)) in overview.iter().enumerate() {
        let row = 4 + idx as u32;
        let value_len = value.chars().count();
        let row_height = if value_len > 110 {
            76.0
        } else if value_len > 70 {
            56.0
        } else if value_len > 38 {
            40.0
        } else {
            32.0
        };
        let _ = sheet.set_row_height(row, row_height);
        sheet
            .write_string_with_format(row, 0, *field, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                row,
                1,
                excel_safe_text(value),
                if *field == "Promotion state" {
                    promotion_fmt
                } else {
                    &wrap_fmt
                },
            )
            .map_err(|e: XlsxError| e.to_string())?;
        let interpretation = match *field {
            "Statistical validation" => {
                "Statistical evidence is separate from operational promotion eligibility."
            }
            "Promotion state" | "Promotion eligible" => {
                "Use the artifact-recorded fail-closed promotion result; human review is still required."
            }
            _ => "Recorded directly from the validation evidence pack when available.",
        };
        sheet
            .write_string_with_format(row, 2, interpretation, &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        3,
        0,
        3 + overview.len() as u32,
        2,
        "ResearchReadinessTable",
        &["Readiness Field", "Recorded Value", "Interpretation"],
        TableStyle::Medium7,
    )?;

    let recorded_funnel =
        pack.and_then(|value| evidence_pack_candidate_funnel(value, model.all_rows.is_empty()));
    let (funnel, funnel_title) =
        if let Some([requested, evaluated, retained, survived, approved]) = recorded_funnel {
            (
                vec![
                    ("Requested", requested),
                    ("Evaluated", evaluated),
                    ("Retained", retained),
                    ("Hard-gate survivors", survived),
                    ("Approved", approved),
                ],
                "Candidate evidence funnel",
            )
        } else {
            let observed_rows = model.all_rows.len();
            let survived = observed_hard_gate_survivors(model).min(observed_rows);
            let approved = model.approved_rows.len().min(survived);
            (
                vec![
                    ("Observed candidate rows", observed_rows),
                    ("Observed hard-gate survivors", survived),
                    ("Observed approved rows", approved),
                ],
                "Observed candidate evidence (pack counts unavailable)",
            )
        };
    sheet
        .write_string_with_format(3, 4, "Candidate Funnel Stage", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .write_string_with_format(3, 5, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (idx, (stage, count)) in funnel.iter().enumerate() {
        let row = 4 + idx as u32;
        sheet
            .write_string_with_format(row, 4, *stage, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_number_with_format(row, 5, *count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        3,
        4,
        3 + funnel.len() as u32,
        5,
        "CandidateFunnelTable",
        &["Candidate Funnel Stage", "Count"],
        TableStyle::Medium2,
    )?;
    let mut funnel_chart = Chart::new(ChartType::Column);
    funnel_chart
        .add_series()
        .set_categories(("Research Readiness", 4, 4, 3 + funnel.len() as u32, 4))
        .set_values(("Research Readiness", 4, 5, 3 + funnel.len() as u32, 5))
        .set_format(ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#2563EB")));
    funnel_chart.title().set_name(funnel_title);
    funnel_chart.legend().set_hidden();
    funnel_chart.x_axis().set_name("Stage");
    funnel_chart
        .y_axis()
        .set_name("Candidates")
        .set_num_format("0")
        .set_major_unit(count_axis_major_unit(
            funnel.iter().map(|(_, count)| *count).max().unwrap_or(0),
        ));
    funnel_chart.set_width(520).set_height(300).set_alt_text(
        "Column chart showing authoritative evidence-pack counts when available, otherwise monotonic observed candidate-row counts.",
    );
    sheet
        .insert_chart_with_offset(3, 7, &funnel_chart, 8, 8)
        .map_err(|e: XlsxError| e.to_string())?;

    let dataset_start = 13u32;
    for (col, header) in ["Dataset Field", "Recorded Value", "Source"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string_with_format(dataset_start, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let evidence_data_id = pack
        .map(|value| value.dataset_data_id.as_str())
        .filter(|value| !value.trim().is_empty());
    let evidence_strategy = pack
        .map(|value| value.dataset_strategy_kind.as_str())
        .filter(|value| !value.trim().is_empty());
    let (data_id, data_id_source) = evidence_data_id
        .map(|value| (value.to_string(), "Validation evidence pack"))
        .unwrap_or_else(|| {
            if model.data_id.trim().is_empty() {
                ("Not recorded in artifact".to_string(), "Not recorded")
            } else {
                (model.data_id.clone(), "Validation result fallback")
            }
        });
    let (strategy, strategy_source) = evidence_strategy
        .map(|value| (value.to_string(), "Validation evidence pack"))
        .unwrap_or_else(|| {
            if model.strategy_kind.trim().is_empty() {
                ("Not recorded in artifact".to_string(), "Not recorded")
            } else {
                (model.strategy_kind.clone(), "Validation result fallback")
            }
        });
    let evidence_only_field = |value: Option<&str>| -> (String, &'static str) {
        value
            .filter(|value| !value.trim().is_empty())
            .map(|value| (value.to_string(), "Validation evidence pack"))
            .unwrap_or_else(|| ("Not recorded in artifact".to_string(), "Not recorded"))
    };
    let (dataset_label, dataset_label_source) =
        evidence_only_field(pack.map(|value| value.dataset_label.as_str()));
    let (dataset_source_file, dataset_source_file_source) =
        evidence_only_field(pack.map(|value| value.dataset_source_file.as_str()));
    let (dataset_timeframe, dataset_timeframe_source) =
        evidence_only_field(pack.map(|value| value.dataset_timeframe.as_str()));
    let dataset_rows = [
        ("Data ID", data_id, data_id_source),
        ("Strategy", strategy, strategy_source),
        ("Label", dataset_label, dataset_label_source),
        (
            "Source file",
            dataset_source_file,
            dataset_source_file_source,
        ),
        ("Timeframe", dataset_timeframe, dataset_timeframe_source),
    ];
    for (idx, (field, value, source)) in dataset_rows.iter().enumerate() {
        let row = dataset_start + 1 + idx as u32;
        sheet
            .write_string_with_format(row, 0, *field, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 1, excel_safe_text(value), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 2, *source, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        dataset_start,
        0,
        dataset_start + dataset_rows.len() as u32,
        2,
        "ResearchDatasetTable",
        &["Dataset Field", "Recorded Value", "Source"],
        TableStyle::Medium4,
    )?;

    let realism_start = 21u32;
    for (col, header) in ["Realism Dimension", "Declared Status", "Interpretation"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string_with_format(realism_start, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let realism_rows = [
        (
            "Price adjustment",
            pack.map(|value| value.realism_price_adjustment.as_str())
                .unwrap_or(""),
        ),
        (
            "Corporate actions",
            pack.map(|value| value.realism_corporate_actions.as_str())
                .unwrap_or(""),
        ),
        (
            "Survivorship bias",
            pack.map(|value| value.realism_survivorship_bias.as_str())
                .unwrap_or(""),
        ),
        (
            "Liquidity / capacity",
            pack.map(|value| value.realism_liquidity_capacity.as_str())
                .unwrap_or(""),
        ),
        (
            "Market impact",
            pack.map(|value| value.realism_market_impact.as_str())
                .unwrap_or(""),
        ),
        (
            "Borrow availability",
            pack.map(|value| value.realism_borrow_availability.as_str())
                .unwrap_or(""),
        ),
    ];
    for (idx, (dimension, raw_status)) in realism_rows.iter().enumerate() {
        let row = realism_start + 1 + idx as u32;
        let status = if raw_status.trim().is_empty() {
            "unknown"
        } else {
            *raw_status
        };
        let status_fmt = match status {
            "adjusted" | "documented" => approved_fmt,
            "not_applicable" => text_fmt,
            _ => rejected_fmt,
        };
        sheet
            .write_string_with_format(row, 0, *dimension, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                row,
                1,
                excel_safe_text(&display_identifier(status)),
                status_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                row,
                2,
                excel_safe_text(realism_interpretation(dimension, status)),
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        realism_start,
        0,
        realism_start + realism_rows.len() as u32,
        2,
        "DeploymentRealismTable",
        &["Realism Dimension", "Declared Status", "Interpretation"],
        TableStyle::Medium3,
    )?;

    let gate_start = 21u32;
    sheet
        .write_string_with_format(gate_start, 4, "Promotion Gate", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .write_string_with_format(gate_start, 5, "Result", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let passed: Vec<&str> = pack
        .map(|value| value.promotion_gates_passed.split("; ").collect())
        .unwrap_or_default();
    let failed: Vec<&str> = pack
        .map(|value| value.promotion_gates_failed.split("; ").collect())
        .unwrap_or_default();
    let mut gates: Vec<(&str, &str)> = passed
        .iter()
        .copied()
        .filter(|value| !value.trim().is_empty())
        .map(|value| (value, "Passed"))
        .collect();
    gates.extend(
        failed
            .iter()
            .copied()
            .filter(|value| !value.trim().is_empty())
            .map(|value| (value, "Failed")),
    );
    if gates.is_empty() {
        gates.push(("Promotion gates not recorded", "Not recorded"));
    }
    for (idx, (gate, result)) in gates.iter().enumerate() {
        let row = gate_start + 1 + idx as u32;
        sheet
            .write_string_with_format(
                row,
                4,
                excel_safe_text(&display_identifier(gate)),
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                row,
                5,
                *result,
                match *result {
                    "Passed" => approved_fmt,
                    "Failed" => rejected_fmt,
                    _ => review_fmt,
                },
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        gate_start,
        4,
        gate_start + gates.len() as u32,
        5,
        "PromotionGatesTable",
        &["Promotion Gate", "Result"],
        TableStyle::Medium3,
    )?;

    let notes_start =
        (realism_start + realism_rows.len() as u32 + 3).max(gate_start + gates.len() as u32 + 3);
    for (offset, (label, value)) in [
        (
            "Missing / unknown realism",
            pack.map(|value| value.realism_missing_or_unknown.as_str())
                .unwrap_or("All realism evidence unavailable"),
        ),
        (
            "Evidence warnings",
            pack.map(|value| value.warnings.as_str())
                .unwrap_or("Evidence pack unavailable"),
        ),
        (
            "Evidence caveats",
            pack.map(|value| value.caveats.as_str()).unwrap_or(
                "Statistical approval is research evidence, not deployment authorization.",
            ),
        ),
    ]
    .iter()
    .enumerate()
    {
        let row = notes_start + offset as u32;
        sheet
            .write_string_with_format(row, 0, *label, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .merge_range(row, 1, row, 6, &excel_safe_text(value), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        let _ = sheet.set_row_height(row, 42.0);
    }
    Ok(())
}

fn build_execution_provenance_sheet(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
) -> Result<(), String> {
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("Execution Provenance")
        .map_err(|e: XlsxError| e.to_string())?;
    sheet.set_zoom(105);
    sheet.set_tab_color(Color::RGB(0x475569));
    sheet.set_screen_gridlines(false);
    apply_shareable_sheet_layout(sheet, "VectorGrid Execution Provenance");
    let _ = sheet.set_freeze_panes(4, 0);
    let wrap_fmt = Format::new()
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);
    for (col, width) in [(0u16, 28.0), (1, 70.0), (2, 50.0), (4, 30.0), (5, 72.0)] {
        let _ = sheet.set_column_width(col, width);
    }
    sheet
        .merge_range(0, 0, 0, 7, "Execution Provenance", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .merge_range(
            1,
            0,
            1,
            7,
            "Artifact-recorded execution is separated from request-time policy expectations. Expected routes are predictions, not runtime proof.",
            &wrap_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let pack = model.validation_evidence_pack.as_ref();
    for (col, header) in [
        "Artifact-Recorded Field",
        "Recorded Value",
        "Evidence Source",
    ]
    .iter()
    .enumerate()
    {
        sheet
            .write_string_with_format(3, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let residency_readout = execution_residency_readout(model);
    let actual_rows: Vec<(&str, String, &str)> = vec![
        (
            "Requested backend",
            if model.requested_backend.trim().is_empty() {
                pack.map(|value| display_identifier(&value.requested_backend))
                    .unwrap_or_else(|| "Not recorded in artifact".to_string())
            } else {
                model.requested_backend.clone()
            },
            "Validation result / evidence pack",
        ),
        (
            "Realized backend",
            if model.backend_used.trim().is_empty() {
                pack.map(|value| display_identifier(&value.realized_backend))
                    .unwrap_or_else(|| "Not recorded in artifact".to_string())
            } else {
                model.backend_used.clone()
            },
            "Validation result / evidence pack",
        ),
        (
            "Realized route",
            if model.backend_route.trim().is_empty() {
                pack.map(|value| display_identifier(&value.route_label))
                    .unwrap_or_else(|| "Not recorded in artifact".to_string())
            } else {
                model.backend_route.clone()
            },
            "Validation result / evidence pack",
        ),
        (
            "Residency tier",
            residency_readout,
            "Validation result / evidence pack + terminal-route consistency check",
        ),
        (
            "Acceleration status",
            if model.validation_acceleration_status.trim().is_empty() {
                "Not recorded in artifact".to_string()
            } else {
                model.validation_acceleration_status.clone()
            },
            "Validation result",
        ),
        (
            "Runtime",
            model
                .runtime_ms
                .map(|value| format!("{value} ms"))
                .unwrap_or_else(|| "Not recorded in artifact".to_string()),
            "Validation result",
        ),
        (
            "Fallback",
            if model.actual_fallback_summary.trim().is_empty()
                || model.actual_fallback_summary.trim() == "-"
            {
                pack.map(|value| value.fallback_reason.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "Not recorded in artifact".to_string())
            } else {
                model.actual_fallback_summary.clone()
            },
            "Validation result / evidence pack",
        ),
        (
            "Route note",
            if model.actual_route_note_summary.trim().is_empty()
                || model.actual_route_note_summary.trim() == "-"
            {
                "-".to_string()
            } else {
                model.actual_route_note_summary.clone()
            },
            "Validation result",
        ),
        (
            "Auxiliary / mixed tails",
            display_identifier(&model.actual_aux_tail_summary),
            "Inferred from request and supported route contract",
        ),
        (
            "Skipped modules",
            display_identifier(&model.actual_skipped_module_summary),
            "Validation result",
        ),
        (
            "Disabled modules",
            display_identifier(&model.actual_disabled_module_summary),
            "Validation request",
        ),
        (
            "Cross-omitted modules",
            display_identifier(&model.actual_cross_omitted_summary),
            "Validation request / mode",
        ),
        (
            "Route diagnostics",
            pack.map(|value| value.route_diagnostics.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "Not recorded in artifact".to_string()),
            "Validation evidence pack",
        ),
    ];
    for (idx, (field, value, source)) in actual_rows.iter().enumerate() {
        let row = 4 + idx as u32;
        let row_height = wrapped_row_height(value, 72, 34.0, 105.0)
            .max(wrapped_row_height(source, 50, 34.0, 105.0));
        let _ = sheet.set_row_height(row, row_height);
        sheet
            .write_string_with_format(row, 0, *field, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 1, excel_safe_text(value), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 2, *source, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        3,
        0,
        3 + actual_rows.len() as u32,
        2,
        "RecordedExecutionTable",
        &[
            "Artifact-Recorded Field",
            "Recorded Value",
            "Evidence Source",
        ],
        TableStyle::Medium4,
    )?;

    for (col, header) in ["Request-Time Policy", "Expected / Inferred Value"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string_with_format(3, 4 + col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let expected_rows = [
        ("Expected route", model.expected_route_summary.as_str()),
        ("Expected cross mode", model.expected_cross_summary.as_str()),
        ("Expected modules", model.expected_module_summary.as_str()),
        (
            "Expected execution categories",
            model.expected_category_summary.as_str(),
        ),
        (
            "Expected fallback",
            model.expected_fallback_summary.as_str(),
        ),
        (
            "Expected route note",
            model.expected_route_note_summary.as_str(),
        ),
        (
            "Expected disabled modules",
            model.expected_disabled_module_summary.as_str(),
        ),
        (
            "Expected cross omissions",
            model.expected_cross_omitted_summary.as_str(),
        ),
        ("Expected warnings", model.expected_warning_summary.as_str()),
        ("Bootstrap policy", model.bootstrap_mode_summary.as_str()),
        ("WFE policy", model.wfe_policy_summary.as_str()),
        ("Friction policy", model.friction_policy_summary.as_str()),
    ];
    for (idx, (field, value)) in expected_rows.iter().enumerate() {
        let row = 4 + idx as u32;
        let _ = sheet.set_row_height(row, wrapped_row_height(value, 76, 38.0, 120.0));
        sheet
            .write_string_with_format(row, 4, *field, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(row, 5, excel_safe_text(value), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        3,
        4,
        3 + expected_rows.len() as u32,
        5,
        "ExpectedExecutionPolicyTable",
        &["Request-Time Policy", "Expected / Inferred Value"],
        TableStyle::Medium7,
    )?;

    let provenance_start = 20u32;
    for (col, header) in ["Validation Provenance", "Recorded Value"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string_with_format(provenance_start, col as u16, *header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let provenance = model.validation_provenance.as_ref();
    let provenance_rows = [
        (
            "Stage sequence",
            provenance
                .map(|value| value.stage_sequence.as_str())
                .unwrap_or(""),
        ),
        (
            "Score channel",
            provenance
                .map(|value| value.score_channel.as_str())
                .unwrap_or(""),
        ),
        (
            "Passed vs approved contract",
            provenance
                .map(|value| value.passed_vs_approved_contract.as_str())
                .unwrap_or(""),
        ),
        (
            "Probability metric source",
            provenance
                .map(|value| value.probability_metric_source.as_str())
                .unwrap_or(""),
        ),
        (
            "Track-record source",
            provenance
                .map(|value| value.track_record_source.as_str())
                .unwrap_or(""),
        ),
        (
            "CSCV coverage",
            provenance
                .map(|value| value.cscv_coverage_state.as_str())
                .unwrap_or(""),
        ),
        (
            "Cross summary scope",
            provenance
                .map(|value| value.cross_summary_scope.as_str())
                .unwrap_or(""),
        ),
        (
            "Regime semantics",
            provenance
                .map(|value| value.regime_summary_semantics.as_str())
                .unwrap_or(""),
        ),
    ];
    for (idx, (field, value)) in provenance_rows.iter().enumerate() {
        let row = provenance_start + 1 + idx as u32;
        sheet
            .write_string_with_format(row, 0, *field, text_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(
                row,
                1,
                excel_safe_text(&display_identifier(value)),
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }
    add_native_table(
        sheet,
        provenance_start,
        0,
        provenance_start + provenance_rows.len() as u32,
        1,
        "ValidationProvenanceTable",
        &["Validation Provenance", "Recorded Value"],
        TableStyle::Medium2,
    )?;
    Ok(())
}

fn build_distributions(
    workbook: &mut Workbook,
    model: &ValidationExportModel,
    title_fmt: &Format,
    header_fmt: &Format,
    text_fmt: &Format,
    num_fmt: &Format,
    pct_fmt: &Format,
    int_fmt: &Format,
) -> Result<(), String> {
    use std::collections::BTreeMap;

    let dist = workbook.add_worksheet();
    dist.set_name("Distributions")
        .map_err(|e: XlsxError| e.to_string())?;
    dist.set_zoom(110);
    dist.set_tab_color(Color::RGB(0xF59E0B));
    dist.set_screen_gridlines(false);
    apply_shareable_sheet_layout(dist, "VectorGrid Validation Distributions");
    let _ = dist.set_freeze_panes(4, 0);
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
    let _ = dist.set_column_width(15, 12);
    let _ = dist.set_column_width(16, 12);
    let _ = dist.set_column_width(18, 13);
    let _ = dist.set_column_width(19, 13);
    dist.merge_range(0, 0, 0, 19, "Distributions", title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(1, 0, "Visible shortlist cap", text_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(1, 1, model.visible_cap as f64, int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(1, 3, "Rows used in distributions", text_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(1, 4, model.all_rows.len() as f64, int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(1, 6, "Metric chart rows", text_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(
        1,
        7,
        model.all_rows.len().min(SUMMARY_CHART_ROW_CAP) as f64,
        int_fmt,
    )
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
            dist.write_string(r, 6, excel_safe_text(reason))
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
    dist.write_string_with_format(3, 15, "Plateau", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 16, "Max Drift", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 18, "WF PnL Eff", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(3, 19, "WF Sharpe Eff", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (idx, row) in model.all_rows.iter().enumerate() {
        let r = 4 + idx as u32;
        dist.write_number_with_format(r, 9, row.rank as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 10, row.robust_score, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 12, row.max_dd, pct_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 13, row.sharpe, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        write_optional_number(dist, r, 15, row.plateau_score, num_fmt)?;
        write_optional_usize(dist, r, 16, row.max_view_rank_drift, int_fmt)?;
        write_optional_number(dist, r, 18, row.wf_pnl_efficiency, num_fmt)?;
        write_optional_number(dist, r, 19, row.wf_sharpe_efficiency, num_fmt)?;
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
        approval_chart
            .set_width(520)
            .set_height(300)
            .set_alt_text("Column chart of candidate counts by approval state.");
        dist.insert_chart_with_offset(3, 21, &approval_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !reason_counts.is_empty() {
        let reason_max = 3 + reason_counts.len() as u32;
        let mut reason_chart = Chart::new(ChartType::Bar);
        reason_chart
            .add_series()
            .set_categories(("Distributions", 4, 6, reason_max, 6))
            .set_values(("Distributions", 4, 7, reason_max, 7));
        reason_chart
            .title()
            .set_name("Top rejection / review reasons");
        reason_chart.legend().set_hidden();
        reason_chart
            .set_width(520)
            .set_height(300)
            .set_alt_text("Bar chart of the most frequent rejection and review reasons.");
        dist.insert_chart_with_offset(3, 30, &reason_chart, 10, 10)
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
        profile_chart
            .set_width(520)
            .set_height(300)
            .set_alt_text("Bar chart of candidate counts by strategy profile.");
        dist.insert_chart_with_offset(20, 21, &profile_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !model.all_rows.is_empty() {
        let row_min = 4u32;
        let row_max = 3 + model.all_rows.len().min(SUMMARY_CHART_ROW_CAP) as u32;
        let mut robust_chart = Chart::new(ChartType::Line);
        robust_chart
            .add_series()
            .set_categories(("Distributions", row_min, 9, row_max, 9))
            .set_values(("Distributions", row_min, 10, row_max, 10));
        robust_chart.title().set_name(&capped_chart_title(
            "Robust score distribution by rank",
            model.all_rows.len(),
        ));
        robust_chart.legend().set_hidden();
        robust_chart
            .set_width(520)
            .set_height(300)
            .set_alt_text("Line chart of robust score across the top candidate ranks.");
        dist.insert_chart_with_offset(20, 30, &robust_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;

        let mut risk_chart = Chart::new(ChartType::Scatter);
        risk_chart
            .add_series()
            .set_categories(("Distributions", row_min, 12, row_max, 12))
            .set_values(("Distributions", row_min, 13, row_max, 13));
        risk_chart
            .title()
            .set_name(&capped_chart_title("Sharpe vs MaxDD", model.all_rows.len()));
        risk_chart.x_axis().set_name("MaxDD").set_num_format("0.0%");
        risk_chart.y_axis().set_name("Sharpe");
        risk_chart.legend().set_hidden();
        risk_chart
            .set_width(520)
            .set_height(300)
            .set_alt_text("Scatter chart comparing Sharpe with maximum drawdown.");
        dist.insert_chart_with_offset(37, 21, &risk_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;

        if model.all_rows.iter().all(|row| row.plateau_score.is_some()) {
            let mut plateau_chart = Chart::new(ChartType::Line);
            plateau_chart
                .add_series()
                .set_categories(("Distributions", row_min, 9, row_max, 9))
                .set_values(("Distributions", row_min, 15, row_max, 15));
            plateau_chart.title().set_name(&capped_chart_title(
                "Plateau score by robust rank",
                model.all_rows.len(),
            ));
            plateau_chart.legend().set_hidden();
            plateau_chart
                .set_width(520)
                .set_height(300)
                .set_alt_text("Line chart of plateau score across candidate ranks.");
            dist.insert_chart_with_offset(37, 30, &plateau_chart, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if model
            .all_rows
            .iter()
            .all(|row| row.max_view_rank_drift.is_some())
        {
            let mut drift_marker = ChartMarker::new();
            drift_marker
                .set_type(ChartMarkerType::Circle)
                .set_size(6)
                .set_format(
                    ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#2563EB")),
                );
            let mut drift_chart = Chart::new(ChartType::Line);
            drift_chart
                .add_series()
                .set_categories(("Distributions", row_min, 9, row_max, 9))
                .set_values(("Distributions", row_min, 16, row_max, 16))
                .set_marker(&drift_marker);
            drift_chart.title().set_name(&capped_chart_title(
                "Max rank drift by robust rank",
                model.all_rows.len(),
            ));
            drift_chart
                .y_axis()
                .set_name("Rank drift")
                .set_min(-0.5)
                .set_num_format("0");
            drift_chart.legend().set_hidden();
            drift_chart
                .set_width(520)
                .set_height(300)
                .set_alt_text("Column chart of maximum ranking drift by candidate rank.");
            dist.insert_chart_with_offset(54, 21, &drift_chart, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if model
            .all_rows
            .iter()
            .all(|row| row.wf_sharpe_efficiency.is_some())
        {
            let mut efficiency_chart = Chart::new(ChartType::Line);
            efficiency_chart
                .add_series()
                .set_categories(("Distributions", row_min, 9, row_max, 9))
                .set_values(("Distributions", row_min, 19, row_max, 19));
            efficiency_chart.title().set_name(&capped_chart_title(
                "WF Sharpe efficiency by robust rank",
                model.all_rows.len(),
            ));
            efficiency_chart.legend().set_hidden();
            efficiency_chart
                .set_width(520)
                .set_height(300)
                .set_alt_text("Line chart of walk-forward Sharpe efficiency by candidate rank.");
            dist.insert_chart_with_offset(54, 30, &efficiency_chart, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;
        }
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
        .set_num_format("0.0%")
        .set_border(FormatBorder::Thin);
    let int_fmt = Format::new()
        .set_num_format("0")
        .set_border(FormatBorder::Thin);
    let approved_fmt = Format::new()
        .set_font_color(Color::RGB(0x166534))
        .set_background_color(Color::RGB(0xDCFCE7))
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let review_fmt = Format::new()
        .set_font_color(Color::RGB(0x92400E))
        .set_background_color(Color::RGB(0xFEF3C7))
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let rejected_fmt = Format::new()
        .set_font_color(Color::RGB(0x991B1B))
        .set_background_color(Color::RGB(0xFEE2E2))
        .set_align(FormatAlign::VerticalCenter)
        .set_border(FormatBorder::Thin);
    let compat_fmt = Format::new()
        .set_font_color(Color::RGB(0x1D4ED8))
        .set_background_color(Color::RGB(0xDBEAFE))
        .set_align(FormatAlign::VerticalCenter)
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
    validate_xlsx_export_limits(&model)?;
    let mut workbook = Workbook::new();
    // rust_xlsxwriter otherwise embeds SystemTime::now() in docProps/core.xml.
    // Bind the metadata to report content so identical reports produce identical
    // XLSX bytes even when they are built on different threads or at different
    // wall-clock times.
    // Legacy reports without a parseable timestamp use a fixed fallback.
    let fallback_creation_time =
        ExcelDateTime::from_ymd(2000, 1, 1).map_err(|e: XlsxError| e.to_string())?;
    let creation_time = report
        .get("ts")
        .and_then(Value::as_str)
        .filter(|timestamp| timestamp.contains('-'))
        .and_then(|timestamp| ExcelDateTime::parse_from_str(timestamp).ok())
        .unwrap_or(fallback_creation_time);
    let properties = DocProperties::new()
        .set_title("VectorGrid Validation Evidence Report")
        .set_subject("Statistical validation, research readiness, and execution provenance")
        .set_author("VectorGrid")
        .set_category("Quantitative Research")
        .set_keywords("validation, backtest, robustness, research evidence, provenance")
        .set_comment(
            "Statistical approval is research evidence and is not deployment authorization.",
        )
        .set_creation_datetime(&creation_time);
    workbook.set_properties(&properties);
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
    let approved_rows = if model.approved_rows.len() > model.visible_cap {
        &model.approved_rows[..model.visible_cap]
    } else {
        &model.approved_rows
    };
    write_candidate_sheet(
        &mut workbook,
        "Approved Candidates",
        approved_rows,
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
        &pct_fmt,
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

    build_validation_features_sheet(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    build_robustness_matrix(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
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

    build_rejection_diagnostics(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
    )?;

    build_report_guide(&mut workbook, &title_fmt, &header_fmt, &text_fmt)?;

    build_candidate_decision_sheet(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &num_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    build_downside_stress_sheet(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
    )?;

    write_regime_audit_sheet(
        &mut workbook,
        &model,
        &header_fmt,
        &text_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
    )?;

    build_candidate_comparison_sheet(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &num_fmt,
        &pct_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
        &compat_fmt,
    )?;

    build_research_readiness_sheet(
        &mut workbook,
        &model,
        &title_fmt,
        &header_fmt,
        &text_fmt,
        &int_fmt,
        &approved_fmt,
        &review_fmt,
        &rejected_fmt,
    )?;

    build_execution_provenance_sheet(&mut workbook, &model, &title_fmt, &header_fmt, &text_fmt)?;

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

    fn core_properties_xml(bytes: &[u8]) -> String {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
        let mut file = zip
            .by_name("docProps/core.xml")
            .expect("core document properties");
        let mut text = String::new();
        file.read_to_string(&mut text)
            .expect("read core document properties");
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

    fn all_xml(bytes: &[u8]) -> String {
        let mut zip = ZipArchive::new(Cursor::new(bytes)).expect("zip archive");
        let mut out = String::new();
        for idx in 0..zip.len() {
            let mut file = zip.by_index(idx).expect("zip entry");
            if !file.name().ends_with(".xml") {
                continue;
            }
            let mut text = String::new();
            file.read_to_string(&mut text).expect("read xml entry");
            out.push_str(&text);
        }
        out
    }

    #[test]
    fn validation_report_workbook_contains_expected_sheets_and_charts() {
        let report = serde_json::json!({
          "version": 1,
          "kind": "vectorgrid_report",
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
        assert!(workbook.contains("Candidate Comparison"));
        assert!(workbook.contains("Research Readiness"));
        assert!(workbook.contains("Execution Provenance"));
        let all_xml = all_xml(&bytes);
        assert!(all_xml.contains("Research only (legacy evidence pack absent)"));
        assert!(all_xml.contains("CandidateComparisonTable"));
        assert!(all_xml.contains("ResearchReadinessTable"));
        assert!(all_xml.contains("RecordedExecutionTable"));
        let core_properties = core_properties_xml(&bytes);
        assert!(core_properties.contains("VectorGrid Validation Evidence Report"));
        assert!(core_properties.contains("not deployment authorization"));
        assert!(core_properties.contains(
            "<dcterms:created xsi:type=\"dcterms:W3CDTF\">2026-03-13T00:00:00Z</dcterms:created>"
        ));
        assert!(core_properties.contains(
            "<dcterms:modified xsi:type=\"dcterms:W3CDTF\">2026-03-13T00:00:00Z</dcterms:modified>"
        ));
    }

    #[test]
    fn validation_report_xlsx_uses_monte_carlo_when_legacy_bootstrap_is_absent() {
        let report = serde_json::json!({
          "version": 1,
          "kind": "vectorgrid_report",
          "ts": "2026-03-13T00:00:00Z",
          "run": { "req": { "strategy_kind": "DoubleMa" } },
          "validation": {
            "req": {
              "monte_carlo": {
                "schema_version": 1,
                "simulations": 500,
                "modes": ["trade_iid"]
              }
            },
            "res": {
              "data_id": "BTCUSD_1h",
              "backend_used": "CpuOnly",
              "backend_route": "CpuOnly",
              "run_verdict": { "passed": true, "approved": true, "approved_candidate_count": 1 },
              "candidates": [{
                "params": { "fast_ma_type": "ema", "fast_len": 10, "slow_ma_type": "sma", "slow_len": 30 },
                "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.2, "trades": 10 },
                "monte_carlo": {
                  "schema_version": 1,
                  "simulations": 500,
                  "trade_count": 10,
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
                    }
                  },
                  "advisory": { "passed": true }
                },
                "validation_verdict": { "passed": true, "warning_flags": [], "fail_reasons": [] },
                "robust_score": 1.23
              }]
            }
          }
        });

        let bytes = build_validation_report_xlsx(&report, Some(25)).expect("xlsx");
        let xml = all_xml(&bytes);
        assert!(xml.contains("iid-only closed-trade bootstrap"));
        assert!(xml.contains("IID only"));
        assert!(xml.contains("MC Prob+"));
        assert!(xml.contains("iid_only"));
        assert!(xml.contains("MC Samples"));
    }

    #[test]
    fn validation_report_empties_approved_sheet_when_run_verdict_rejects_final_approval() {
        let report = serde_json::json!({
          "version": 1,
          "kind": "vectorgrid_report",
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
          "kind": "vectorgrid_report",
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

    #[test]
    fn validation_report_surfaces_fail_closed_research_and_promotion_evidence() {
        let mut report = serde_json::json!({
          "version": 1,
          "kind": "vectorgrid_report",
          "ts": "2026-03-13T00:00:00Z",
          "run": { "req": { "strategy_kind": "CompositeTrend" } },
          "validation": {
            "req": {
              "backend": { "backend": "Auto" },
              "candidates": [{
                "strategy": { "trend_quality_use_adx": false }
              }]
            },
            "res": {
              "data_id": "research-dataset",
              "requested_backend": "Auto",
              "backend_used": "Gpu",
              "backend_route": "Auto->GpuKernel",
              "validation_residency_tier": "gpu_kernel",
              "runtime_ms": 12,
              "run_verdict": {
                "passed": true,
                "approved": true,
                "approved_candidate_count": 1,
                "candidates_tested": 1,
                "candidates_survived_hard_gates": 1,
                "final_recommendation": "approved_with_warnings"
              },
              "validation_evidence_pack": {
                "schema_version": "1-1-0",
                "version": 1,
                "workflow": "double_ma_validation",
                "status": "incomplete",
                "status_reason": "deployment-realism evidence unknown",
                "generated_by": "ta_app_core.validation_pipeline",
                "dataset": {
                  "data_id": "research-dataset",
                  "strategy_kind": "CompositeTrend",
                  "csv_path": "C:\\private\\research.csv",
                  "timeframe": "1h",
                  "label": "Research Dataset",
                  "data_realism": {
                    "price_adjustment": "adjusted",
                    "corporate_actions": "documented",
                    "survivorship_bias": "unknown",
                    "liquidity_capacity": "unknown",
                    "market_impact": "unknown",
                    "borrow_availability": "not_applicable",
                    "complete": false,
                    "missing_or_unknown": [
                      "survivorship_bias",
                      "liquidity_capacity",
                      "market_impact"
                    ]
                  }
                },
                "route": {
                  "requested_backend": "Auto",
                  "realized_backend": "Gpu",
                  "route_label": "Auto->GpuKernel",
                  "residency_tier": "gpu_kernel",
                  "fallback_reason": "hybrid kernel unavailable; kernel fallback used",
                  "diagnostics": ["compact results returned"]
                },
                "candidate_universe": {
                  "source": "validation_request_candidates",
                  "requested_candidates": 1,
                  "evaluated_candidates": 1,
                  "retained_candidates": 1,
                  "survived_hard_gates": 1,
                  "approved_candidates": 1
                },
                "verdict": {
                  "passed": true,
                  "approved": true,
                  "recommendation": "approved_with_warnings",
                  "summary": "approved"
                },
                "promotion_eligibility": {
                  "eligible": false,
                  "state": "research_only",
                  "gates_passed": ["validation_run_approved"],
                  "gates_failed": [
                    "realism_survivorship_bias_declared",
                    "realism_liquidity_capacity_declared",
                    "realism_market_impact_declared"
                  ],
                  "reason": "promotion blocked until missing realism evidence is declared"
                },
                "warnings": ["promotion_blocked_missing_realism_evidence"],
                "caveats": [
                  "validation approval is research evidence, not deployment authorization"
                ]
              },
              "candidates": [{
                "params": {
                  "fast_ma_type": "ema",
                  "fast_len": 10,
                  "slow_ma_type": "sma",
                  "slow_len": 30
                },
                "in_sample": {
                  "pnl": 1.0,
                  "sharpe": 2.0,
                  "max_dd": 0.2,
                  "trades": 10,
                  "exposure": 0.5
                },
                "psr": 0.82,
                "dsr": 0.78,
                "validation_verdict": {
                  "passed": true,
                  "warning_flags": [],
                  "fail_reasons": []
                },
                "robust_score": 1.23,
                "robust_pareto": true
              }]
            }
          }
        });

        let bytes = build_validation_report_xlsx(&report, Some(25)).expect("xlsx");
        let xml = all_xml(&bytes);
        for expected in [
            "Research Readiness",
            "Research Dataset",
            "research.csv",
            "Research only",
            "promotion blocked until missing realism evidence is declared",
            "survivorship bias",
            "Unknown or absent evidence blocks promotion beyond research use.",
            "Auto-&gt;GpuKernel",
            "hybrid kernel unavailable; kernel fallback used",
            "Expected GPU kernel route (CompositeTrend) when available",
            "Statistical approval is research evidence and is not deployment authorization.",
        ] {
            assert!(xml.contains(expected), "missing workbook text: {expected}");
        }
        assert!(!xml.contains("C:\\private\\research.csv"));
        assert!(xml.contains("operator=\"greaterThan\""));
        assert!(xml.contains("operator=\"lessThan\""));
        assert!(xml.contains("operator=\"equal\""));
        assert!(xml.contains("formatCode=\"0.0%\""));

        report["validation"]["res"]["validation_evidence_pack"]["candidate_universe"] = serde_json::json!({
            "requested_candidates": 0,
            "evaluated_candidates": 0,
            "retained_candidates": 0,
            "survived_hard_gates": 0,
            "approved_candidates": 0
        });
        let fallback_bytes =
            build_validation_report_xlsx(&report, Some(25)).expect("fallback funnel xlsx");
        let fallback_xml = all_xml(&fallback_bytes);
        assert!(fallback_xml.contains("Observed candidate rows"));
        assert!(!fallback_xml.contains("Retained shortlist"));

        report["validation"]["res"]["validation_evidence_pack"]["dataset"]["data_realism"]
            ["survivorship_bias"] = serde_json::json!("documented");
        report["validation"]["res"]["validation_evidence_pack"]["dataset"]["data_realism"]
            ["liquidity_capacity"] = serde_json::json!("documented");
        report["validation"]["res"]["validation_evidence_pack"]["dataset"]["data_realism"]
            ["market_impact"] = serde_json::json!("documented");
        report["validation"]["res"]["validation_evidence_pack"]["dataset"]["data_realism"]
            ["complete"] = serde_json::json!(true);
        report["validation"]["res"]["validation_evidence_pack"]["dataset"]["data_realism"]
            ["missing_or_unknown"] = serde_json::json!([]);
        report["validation"]["res"]["validation_evidence_pack"]["promotion_eligibility"]
            ["eligible"] = serde_json::json!(true);
        report["validation"]["res"]["validation_evidence_pack"]["promotion_eligibility"]["state"] =
            serde_json::json!("eligible_for_review");
        report["validation"]["res"]["validation_evidence_pack"]["promotion_eligibility"]
            ["gates_failed"] = serde_json::json!([]);
        report["validation"]["res"]["validation_evidence_pack"]["status"] =
            serde_json::json!("complete");
        let eligible_bytes =
            build_validation_report_xlsx(&report, Some(25)).expect("eligible review xlsx");
        let eligible_xml = all_xml(&eligible_bytes);
        assert!(eligible_xml.contains("Eligible for review"));
        assert!(eligible_xml.contains("eligible for human review"));

        report["validation"]["res"]["run_verdict"]["approved"] = serde_json::json!(false);
        let conflicting_bytes =
            build_validation_report_xlsx(&report, Some(25)).expect("conflicting verdict xlsx");
        let conflicting_xml = all_xml(&conflicting_bytes);
        assert!(conflicting_xml.contains(
            "Research only (fail-closed; top-level run verdict conflicts with evidence pack)"
        ));
        assert!(conflicting_xml.contains(
            "No — fail-closed because the top-level run verdict conflicts with the evidence pack"
        ));
    }

    #[test]
    fn validation_report_checks_pack_only_route_against_pack_residency() {
        let report = serde_json::json!({
            "version": 1,
            "kind": "vectorgrid_report",
            "ts": "2026-03-13T00:00:00Z",
            "run": { "req": { "strategy_kind": "CompositeTrend" } },
            "validation": {
                "res": {
                    "data_id": "pack-only-route",
                    "validation_evidence_pack": {
                        "schema_version": ta_app_core::VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION,
                        "version": ta_app_core::VALIDATION_EVIDENCE_PACK_VERSION,
                        "workflow": "double_ma_validation",
                        "status": "complete",
                        "status_reason": "complete evidence ready for human review",
                        "dataset": {
                            "data_id": "pack-only-route",
                            "strategy_kind": "CompositeTrend",
                            "data_realism": {
                                "price_adjustment": "adjusted",
                                "corporate_actions": "documented",
                                "survivorship_bias": "documented",
                                "liquidity_capacity": "documented",
                                "market_impact": "documented",
                                "borrow_availability": "not_applicable",
                                "complete": true
                            }
                        },
                        "route": {
                            "requested_backend": "Auto",
                            "realized_backend": "Gpu",
                            "route_label": "GpuHybridKernel->GpuKernel",
                            "residency_tier": "gpu_hybrid_kernel"
                        },
                        "verdict": {
                            "passed": true,
                            "approved": true,
                            "summary": "approved"
                        },
                        "promotion_eligibility": {
                            "eligible": true,
                            "state": "eligible_for_review",
                            "reason": "complete evidence ready for review"
                        }
                    },
                    "candidates": []
                }
            }
        });

        let bytes = build_validation_report_xlsx(&report, Some(25)).expect("pack-only route xlsx");
        let xml = all_xml(&bytes);

        assert!(xml.contains("GpuHybridKernel-&gt;GpuKernel"));
        assert!(
            xml.contains("terminal route GpuKernel implies gpu_kernel; provenance mismatch"),
            "pack-only route must receive the same residency consistency check as a result route"
        );
        assert!(xml.contains("Approved as research evidence (evidence pack)"));
        assert!(xml.contains("Yes — eligible for human review"));
    }

    #[test]
    fn validation_report_truncates_oversized_artifact_text_in_new_audit_sheets() {
        let oversized = "🧪".repeat(XLSX_CELL_CHAR_LIMIT + 16);
        let report = serde_json::json!({
            "version": 1,
            "kind": "vectorgrid_report",
            "ts": "2026-03-13T00:00:00Z",
            "run": { "req": { "strategy_kind": "DoubleMa" } },
            "validation": {
                "res": {
                    "data_id": "oversized-text",
                    "validation_evidence_pack": {
                        "schema_version": ta_app_core::VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION,
                        "version": ta_app_core::VALIDATION_EVIDENCE_PACK_VERSION,
                        "workflow": "double_ma_validation",
                        "status": "incomplete",
                        "status_reason": oversized,
                        "dataset": {
                            "data_id": "oversized-text",
                            "strategy_kind": "DoubleMa",
                            "data_realism": {
                                "price_adjustment": oversized,
                                "complete": false,
                                "missing_or_unknown": [oversized]
                            }
                        },
                        "route": {
                            "requested_backend": "CpuOnly",
                            "realized_backend": "Cpu",
                            "route_label": "CpuOnly",
                            "residency_tier": "cpu",
                            "diagnostics": [oversized]
                        },
                        "candidate_universe": {
                            "requested_candidates": 0,
                            "evaluated_candidates": 0,
                            "retained_candidates": 0,
                            "survived_hard_gates": 0,
                            "approved_candidates": 0
                        },
                        "verdict": {
                            "passed": false,
                            "approved": false,
                            "summary": oversized
                        },
                        "promotion_eligibility": {
                            "eligible": false,
                            "state": "research_only",
                            "reason": oversized,
                            "gates_failed": [oversized]
                        },
                        "warnings": [oversized],
                        "caveats": [oversized]
                    },
                    "validation_provenance": {
                        "stage_sequence": [oversized],
                        "score_channel": oversized,
                        "passed_vs_approved_contract": oversized
                    },
                    "candidates": [{
                        "params": {
                            "fast_ma_type": "ema",
                            "fast_len": 10,
                            "slow_ma_type": "sma",
                            "slow_len": 30
                        },
                        "in_sample": {
                            "pnl": 0.0,
                            "sharpe": 0.0,
                            "max_dd": 0.0,
                            "trades": 0,
                            "exposure": 0.0
                        },
                        "validation_verdict": {
                            "passed": false,
                            "warning_flags": [],
                            "fail_reasons": [oversized]
                        },
                        "robust_score": 0.0,
                        "robust_pareto": false
                    }]
                }
            }
        });

        let bytes = build_validation_report_xlsx(&report, Some(25))
            .expect("oversized artifact text should be truncated, not reject XLSX export");
        let xml = all_xml(&bytes);
        assert!(xml.contains(TRUNCATION_SUFFIX));
        assert!(!xml.contains(&oversized));
    }

    #[test]
    fn excel_safe_text_truncates_on_unicode_scalar_boundaries() {
        let original = "🚀".repeat(XLSX_CELL_CHAR_LIMIT + 8);
        let safe = excel_safe_text(&original);
        assert_eq!(safe.chars().count(), XLSX_CELL_CHAR_LIMIT);
        assert!(safe.ends_with(TRUNCATION_SUFFIX));
        assert!(safe.is_char_boundary(safe.len()));
    }

    #[test]
    fn worksheet_row_budget_checks_combined_layout_and_overflow() {
        assert!(validate_sheet_row_budget("Boundary", &[7, XLSX_MAX_ROW_INDEX - 7]).is_ok());
        assert!(validate_sheet_row_budget("Too large", &[7, XLSX_MAX_ROW_INDEX - 6]).is_err());
        assert!(
            validate_sheet_row_budget("Candidate boundary", &[3, XLSX_MAX_ROW_INDEX - 3]).is_ok()
        );
        assert!(
            validate_sheet_row_budget("Candidate overflow", &[3, XLSX_MAX_ROW_INDEX - 2]).is_err()
        );
        assert!(validate_sheet_row_budget("Overflow", &[usize::MAX, 1]).is_err());
        assert_eq!(count_axis_major_unit(0), 1.0);
        assert_eq!(count_axis_major_unit(5), 1.0);
        assert_eq!(count_axis_major_unit(6), 2.0);
        assert_eq!(count_axis_major_unit(10_000), 2_000.0);
        assert_eq!(capped_chart_title("Metric", 100), "Metric");
        assert_eq!(capped_chart_title("Metric", 101), "Metric (top 100)");
        assert_eq!(wrapped_row_height("short", 80, 20.0, 120.0), 21.0);
        assert!(wrapped_row_height(&"x".repeat(300), 80, 20.0, 120.0) >= 66.0);
    }

    #[test]
    fn promotion_evidence_eligibility_is_fail_closed() {
        let mut pack = ExportValidationEvidencePack {
            schema_version: ta_app_core::VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION.to_string(),
            version: Some(u64::from(ta_app_core::VALIDATION_EVIDENCE_PACK_VERSION)),
            status: "complete".to_string(),
            verdict_passed: Some(true),
            verdict_approved: Some(true),
            promotion_eligible: Some(true),
            promotion_state: "eligible_for_review".to_string(),
            promotion_reason: "complete evidence ready for review".to_string(),
            realism_complete: Some(false),
            realism_missing_or_unknown_recorded: false,
            realism_price_adjustment: "adjusted".to_string(),
            realism_corporate_actions: "documented".to_string(),
            realism_survivorship_bias: "documented".to_string(),
            realism_liquidity_capacity: "documented".to_string(),
            realism_market_impact: "documented".to_string(),
            realism_borrow_availability: "not_applicable".to_string(),
            ..ExportValidationEvidencePack::default()
        };
        assert!(!promotion_evidence_is_eligible_for_review(&pack));

        pack.realism_complete = Some(true);
        pack.promotion_gates_failed = "market_impact_declared".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));

        pack.promotion_gates_failed.clear();
        assert!(
            promotion_evidence_is_eligible_for_review(&pack),
            "the canonical serializer omits an empty missing_or_unknown list"
        );

        pack.realism_missing_or_unknown = "market_impact".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.realism_missing_or_unknown.clear();
        pack.realism_market_impact = "unknown".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.realism_market_impact = "invalid".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.realism_market_impact = "documented".to_string();
        pack.verdict_approved = Some(false);
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.verdict_approved = Some(true);
        pack.status = "incomplete".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.status = "complete".to_string();
        pack.schema_version = "future-schema".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.schema_version = ta_app_core::VALIDATION_EVIDENCE_PACK_SCHEMA_VERSION.to_string();
        pack.version = Some(u64::from(ta_app_core::VALIDATION_EVIDENCE_PACK_VERSION) + 1);
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
        pack.version = Some(u64::from(ta_app_core::VALIDATION_EVIDENCE_PACK_VERSION));
        pack.promotion_state = "research_only".to_string();
        assert!(!promotion_evidence_is_eligible_for_review(&pack));
    }

    #[test]
    fn evidence_pack_funnel_requires_complete_monotonic_counts() {
        let mut pack = ExportValidationEvidencePack::default();
        assert!(evidence_pack_candidate_funnel(&pack, false).is_none());

        pack.requested_candidates = Some(4);
        pack.evaluated_candidates = Some(4);
        pack.retained_candidates = Some(2);
        pack.survived_hard_gates = Some(2);
        pack.approved_candidates = Some(1);
        assert_eq!(
            evidence_pack_candidate_funnel(&pack, false),
            Some([4, 4, 2, 2, 1])
        );

        pack.retained_candidates = Some(5);
        assert!(evidence_pack_candidate_funnel(&pack, false).is_none());

        pack.requested_candidates = Some(0);
        pack.evaluated_candidates = Some(0);
        pack.retained_candidates = Some(0);
        pack.survived_hard_gates = Some(0);
        pack.approved_candidates = Some(0);
        assert!(evidence_pack_candidate_funnel(&pack, false).is_none());
        assert_eq!(
            evidence_pack_candidate_funnel(&pack, true),
            Some([0, 0, 0, 0, 0])
        );
    }

    #[test]
    fn disabled_or_omitted_status_surfaces_skipped_summary_entries() {
        let model = ValidationExportModel {
            actual_skipped_module_summary:
                "cscv_pbo: cscv_skipped_not_enough_bars | spa: spa_skipped_not_enough_slices"
                    .to_string(),
            ..ValidationExportModel::default()
        };

        assert_eq!(disabled_or_omitted_status(&model, "cscv_pbo"), "skipped");
        assert_eq!(disabled_or_omitted_status(&model, "spa"), "skipped");
        assert_eq!(
            disabled_or_omitted_status(&model, "cross_ticker"),
            "not run"
        );
    }

    #[test]
    fn internal_sheet_link_quotes_sheet_names_with_spaces() {
        assert_eq!(
            internal_sheet_link("Approved Candidates", "A1"),
            "internal:'Approved Candidates'!A1"
        );
        assert_eq!(
            internal_sheet_link("Audit Data", "A1"),
            "internal:'Audit Data'!A1"
        );
        assert_eq!(
            internal_sheet_link("Shortlist", "A1"),
            "internal:Shortlist!A1"
        );
    }

    #[test]
    fn full_stack_cross_mode_has_specific_dashboard_labels() {
        assert_eq!(display_cross_mode_label("full_stack"), "Full Stack");
        assert_eq!(
            display_cross_semantics_label("full_stack"),
            "full-stack finalist validation"
        );
    }
}
