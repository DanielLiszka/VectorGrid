use rust_xlsxwriter::{
    Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartSolidFill, ChartType,
    ConditionalFormat3ColorScale, ConditionalFormatType, Table, TableColumn, TableStyle, Worksheet,
};
use std::collections::{BTreeMap, BTreeSet};
use ta_app_core::{MetaSelectionResult, TrainDatasetRow};

use super::super::commands_campaign::ResearchCampaignFixedCandidateAggregate;
use super::super::commands_meta::{
    MetaCampaignCohortLineage, MetaCohortManifest, MetaHoldoutExecutionEvidence,
};

const XLSX_MAX_ROW_INDEX: usize = 1_048_575;
const XLSX_MAX_COLUMN_INDEX: usize = 16_383;
const XLSX_CELL_CHAR_LIMIT: usize = 32_767;
const ASSET_MATRIX_HEADER_CHAR_LIMIT: usize = 72;
const ASSET_MATRIX_CELL_BUDGET: usize = 2_000_000;
const SUMMARY_CHART_ROW_CAP: usize = 100;
const HOLDOUT_EXECUTION_BASE_GROUP_HEADER_ROW: usize = 17;
const HOLDOUT_EXECUTION_ROUTE_DATA_FIRST_ROW: usize = 3;
const COHORT_PROVENANCE_ARTIFACT_HEADER_ROW: usize = 17;
const VCET_CANDIDATE_EVIDENCE_HEADER_ROW: usize = 19;
const VCET_CANDIDATE_EVIDENCE_ROW_CAP: usize = ta_app_core::vcet_search::MAX_VCET_SEARCH_BUDGET;
const VCET_CANDIDATE_CHART_ROW_CAP: usize = 100;
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

fn concise_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(1);
    let mut out: String = value.chars().take(keep).collect();
    out.push('…');
    out
}

fn write_finite_number_or_dash(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: f64,
    format: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    if value.is_finite() {
        worksheet
            .write_number_with_format(row, col, value, format)
            .map_err(|e: XlsxError| e.to_string())?;
    } else {
        worksheet
            .write_string(row, col, "-")
            .map_err(|e: XlsxError| e.to_string())?;
    }
    Ok(())
}

fn write_optional_finite_number_or_dash(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<f64>,
    format: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    match value.filter(|value| value.is_finite()) {
        Some(value) => worksheet
            .write_number_with_format(row, col, value, format)
            .map_err(|e: XlsxError| e.to_string())?,
        None => worksheet
            .write_string(row, col, "-")
            .map_err(|e: XlsxError| e.to_string())?,
    };
    Ok(())
}

fn write_optional_integer_or_dash(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<u16>,
    format: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    match value {
        Some(value) => worksheet
            .write_number_with_format(row, col, f64::from(value), format)
            .map_err(|e: XlsxError| e.to_string())?,
        None => worksheet
            .write_string(row, col, "-")
            .map_err(|e: XlsxError| e.to_string())?,
    };
    Ok(())
}

fn write_optional_bool_or_dash(
    worksheet: &mut Worksheet,
    row: u32,
    col: u16,
    value: Option<bool>,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    worksheet
        .write_string(
            row,
            col,
            match value {
                Some(true) => "Yes",
                Some(false) => "No",
                None => "-",
            },
        )
        .map_err(|e: XlsxError| e.to_string())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_vcet_candidate_cohort_metrics(
    worksheet: &mut Worksheet,
    row: u32,
    start_col: u16,
    aggregate: Option<&ResearchCampaignFixedCandidateAggregate>,
    int_fmt: &rust_xlsxwriter::Format,
    pct_fmt: &rust_xlsxwriter::Format,
    pnl_fmt: &rust_xlsxwriter::Format,
    num6_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    let Some(aggregate) = aggregate else {
        for col in start_col..start_col + 13 {
            worksheet
                .write_string(row, col, "-")
                .map_err(|e: XlsxError| e.to_string())?;
        }
        return Ok(());
    };

    for (offset, value) in [
        aggregate.requested_datasets,
        aggregate.evaluated_datasets,
        aggregate.missing_or_failed_datasets,
        aggregate.profitable_datasets,
        aggregate.hard_gate_pass_datasets,
    ]
    .into_iter()
    .enumerate()
    {
        worksheet
            .write_number_with_format(row, start_col + offset as u16, value as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    write_finite_number_or_dash(
        worksheet,
        row,
        start_col + 5,
        aggregate.profitable_share_requested,
        pct_fmt,
    )?;
    write_finite_number_or_dash(
        worksheet,
        row,
        start_col + 6,
        aggregate.hard_gate_pass_share_requested,
        pct_fmt,
    )?;
    write_optional_finite_number_or_dash(
        worksheet,
        row,
        start_col + 7,
        aggregate.pnl_p10,
        pnl_fmt,
    )?;
    write_optional_finite_number_or_dash(
        worksheet,
        row,
        start_col + 8,
        aggregate.pnl_median,
        pnl_fmt,
    )?;
    write_optional_finite_number_or_dash(
        worksheet,
        row,
        start_col + 9,
        aggregate.sharpe_p10,
        num6_fmt,
    )?;
    write_optional_finite_number_or_dash(
        worksheet,
        row,
        start_col + 10,
        aggregate.sharpe_median,
        num6_fmt,
    )?;
    write_optional_finite_number_or_dash(
        worksheet,
        row,
        start_col + 11,
        aggregate.max_drawdown_p90,
        pct_fmt,
    )?;
    write_optional_finite_number_or_dash(
        worksheet,
        row,
        start_col + 12,
        aggregate.trades_median,
        num6_fmt,
    )?;
    Ok(())
}

fn path_basename(value: &str) -> &str {
    value
        .rsplit(|ch| ch == '/' || ch == '\\')
        .find(|part| !part.trim().is_empty())
        .unwrap_or(value)
}

fn share_safe_dataset_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "Dataset".to_string();
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        path_basename(trimmed).to_string()
    } else {
        trimmed.to_string()
    }
}

fn share_safe_dataset_key(value: &str) -> String {
    let trimmed = value.trim();
    let (prefix, payload) = trimmed.split_once(':').unwrap_or(("", trimmed));
    let payload = if payload.contains('/') || payload.contains('\\') {
        path_basename(payload)
    } else {
        payload
    };
    if prefix.is_empty() {
        payload.to_string()
    } else {
        format!("{prefix}:{payload}")
    }
}

fn stable_dataset_identity_suffix(dataset: &str, dataset_key: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in dataset
        .as_bytes()
        .iter()
        .chain(std::iter::once(&0u8))
        .chain(dataset_key.as_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn validate_sheet_row_budget(
    sheet: &str,
    header_row: usize,
    data_rows: usize,
) -> Result<(), String> {
    let last_row = header_row
        .checked_add(data_rows)
        .ok_or_else(|| format!("{sheet} worksheet row calculation overflowed"))?;
    if last_row > XLSX_MAX_ROW_INDEX {
        return Err(format!(
            "{sheet} worksheet requires zero-based row index {last_row}; Excel supports at most {XLSX_MAX_ROW_INDEX}"
        ));
    }
    Ok(())
}

fn holdout_execution_group_header_row(route_rows: usize) -> Result<usize, String> {
    HOLDOUT_EXECUTION_ROUTE_DATA_FIRST_ROW
        .checked_add(route_rows)
        .map(|row| row.max(HOLDOUT_EXECUTION_BASE_GROUP_HEADER_ROW))
        .ok_or_else(|| "holdout execution route row calculation overflowed".to_string())
}

fn asset_matrix_last_column(dataset_columns: usize) -> Result<u16, String> {
    2usize
        .checked_add(dataset_columns)
        .filter(|last_col| *last_col <= XLSX_MAX_COLUMN_INDEX)
        .map(|last_col| last_col as u16)
        .ok_or_else(|| {
            format!(
                "asset matrix contains {dataset_columns} dataset columns; Excel supports at most {} in this layout",
                XLSX_MAX_COLUMN_INDEX - 2
            )
        })
}

fn validate_asset_matrix_cell_budget(
    group_rows: usize,
    dataset_columns: usize,
) -> Result<(), String> {
    if group_rows == 0 || dataset_columns == 0 {
        return Ok(());
    }
    let rows_with_header = group_rows
        .checked_add(1)
        .ok_or_else(|| "asset matrix row calculation overflowed".to_string())?;
    let columns_with_rank = dataset_columns
        .checked_add(3)
        .ok_or_else(|| "asset matrix column calculation overflowed".to_string())?;
    let cells = rows_with_header
        .checked_mul(columns_with_rank)
        .ok_or_else(|| "asset matrix cell calculation overflowed".to_string())?;
    if cells > ASSET_MATRIX_CELL_BUDGET {
        return Err(format!(
            "asset matrix would require {cells} cells ({group_rows} groups × {dataset_columns} datasets plus headers); the XLSX safety budget is {ASSET_MATRIX_CELL_BUDGET} cells"
        ));
    }
    Ok(())
}

fn validate_meta_xlsx_limits(
    meta: &MetaSelectionResult,
    holdout_execution: Option<&MetaHoldoutExecutionEvidence>,
    cohort_manifest: Option<&MetaCohortManifest>,
) -> Result<(), String> {
    validate_sheet_row_budget("Meta Analysis", 6, meta.groups.len())?;
    let train_rows = meta.groups.iter().try_fold(0usize, |count, group| {
        count.checked_add(group.train_rows.len())
    });
    let train_rows = train_rows.ok_or_else(|| "meta training row count overflowed".to_string())?;
    validate_sheet_row_budget("By Asset (Train)", 2, train_rows)?;
    let holdout_rows = meta.groups.iter().try_fold(0usize, |count, group| {
        count.checked_add(group.holdout_rows.len())
    });
    let holdout_rows =
        holdout_rows.ok_or_else(|| "meta holdout row count overflowed".to_string())?;
    validate_sheet_row_budget("Holdout Rows", 2, holdout_rows)?;
    if let Some(execution) = holdout_execution {
        let group_header_row =
            holdout_execution_group_header_row(execution.route_validation_batch_counts.len())?;
        validate_sheet_row_budget(
            "Holdout Execution",
            group_header_row,
            execution.group_metrics.len().max(1),
        )?;
        validate_sheet_row_budget(
            "Holdout Execution routes",
            2,
            execution.route_validation_batch_counts.len().max(1),
        )?;
    }
    if let Some(manifest) = cohort_manifest {
        let artifact_rows = manifest
            .train_artifacts
            .len()
            .checked_add(manifest.holdout_artifacts.len())
            .ok_or_else(|| "cohort provenance artifact row count overflowed".to_string())?;
        validate_sheet_row_budget(
            "Cohort Provenance",
            COHORT_PROVENANCE_ARTIFACT_HEADER_ROW,
            artifact_rows.max(1),
        )?;
        if let Some(evidence) = manifest
            .campaign_lineage
            .as_ref()
            .and_then(|lineage| lineage.vcet_candidate_evidence.as_ref())
        {
            if evidence.candidates.len() > VCET_CANDIDATE_EVIDENCE_ROW_CAP
                || evidence.row_cap > VCET_CANDIDATE_EVIDENCE_ROW_CAP
                || evidence.candidates.len() > evidence.row_cap
            {
                return Err(format!(
                    "VCET candidate evidence contains {} rows with declared cap {}; the XLSX safety cap is {VCET_CANDIDATE_EVIDENCE_ROW_CAP}",
                    evidence.candidates.len(), evidence.row_cap
                ));
            }
            if evidence.included_candidate_count != evidence.candidates.len()
                || evidence.included_candidate_count > evidence.source_candidate_count
                || evidence
                    .included_candidate_count
                    .saturating_add(evidence.omitted_candidate_count)
                    != evidence.source_candidate_count
            {
                return Err(
                    "VCET candidate evidence row-count metadata is internally inconsistent"
                        .to_string(),
                );
            }
            validate_sheet_row_budget(
                "VCET Candidate Evidence",
                VCET_CANDIDATE_EVIDENCE_HEADER_ROW,
                evidence.candidates.len().max(1),
            )?;
        }
    }

    let collision_labels = matrix_dataset_collision_labels(&meta.groups);
    let dataset_columns = meta
        .groups
        .iter()
        .flat_map(|group| group.train_rows.iter())
        .map(|row| matrix_dataset_label(row, &collision_labels))
        .collect::<BTreeSet<_>>()
        .len();
    asset_matrix_last_column(dataset_columns)?;
    validate_asset_matrix_cell_budget(meta.groups.len(), dataset_columns)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_vcet_candidate_evidence_sheet(
    workbook: &mut rust_xlsxwriter::Workbook,
    lineage: &MetaCampaignCohortLineage,
    title_fmt: &rust_xlsxwriter::Format,
    header_fmt: &rust_xlsxwriter::Format,
    label_fmt: &rust_xlsxwriter::Format,
    wrap_fmt: &rust_xlsxwriter::Format,
    int_fmt: &rust_xlsxwriter::Format,
    num6_fmt: &rust_xlsxwriter::Format,
    pct_fmt: &rust_xlsxwriter::Format,
) -> Result<(), String> {
    use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, XlsxError};

    let sheet = workbook.add_worksheet();
    sheet
        .set_name("VCET Candidate Evidence")
        .map_err(|e: XlsxError| e.to_string())?;
    apply_shareable_sheet_layout(sheet, "VectorGrid VCET Candidate Evidence");
    sheet.set_print_fit_to_pages(2, 0);

    let last_col = 45u16;
    sheet
        .merge_range(
            0,
            0,
            0,
            last_col,
            "VCET candidate controls and cross-universe evidence",
            title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let included = lineage
        .vcet_candidate_evidence
        .as_ref()
        .map(|evidence| evidence.included_candidate_count)
        .unwrap_or(0);
    let omitted = lineage
        .vcet_candidate_evidence
        .as_ref()
        .map(|evidence| evidence.omitted_candidate_count)
        .unwrap_or(0);
    let note = if lineage.vcet_candidate_evidence.is_some() {
        format!(
            "Selection uses the training cohort. Holdout and all-universe columns are evidence only. PnL p10/median remain in native strategy PnL units and are not converted to percentages. Search provenance distinguishes QMC from exhaustive enumeration; lexicographic fallback is a legacy counter and is expected to remain zero for current searches. Missing metrics are shown as '-'. The report includes {included} candidate row(s) and omits {omitted} beyond the bounded report projection; selected and admitted candidates are prioritized."
        )
    } else {
        "Candidate evidence is unavailable in this cohort lineage. This usually means the report was generated from a legacy Meta artifact that predates sealed VCET candidate-evidence projection. No parameter or metric values are inferred.".to_string()
    };
    sheet
        .merge_range(1, 0, 1, last_col, &excel_safe_text(&note), wrap_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let note_lines = note.chars().count().saturating_sub(1) / 180 + 1;
    let _ = sheet.set_row_height(1, (30.0 + note_lines.min(4) as f64 * 14.0).min(90.0));

    for (col, header) in [
        (0u16, "Candidate evidence"),
        (3u16, "Search provenance"),
        (6u16, "Admission policy"),
    ] {
        sheet
            .write_string_with_format(2, col, header, header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string_with_format(2, col + 1, "Value", header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if let Some(evidence) = lineage.vcet_candidate_evidence.as_ref() {
        let evidence_labels = [
            "Schema version",
            "Candidate manifest hash",
            "Selection cohort",
            "Selection status",
            "GPU kernel required",
            "Source candidates",
            "Included rows",
            "Omitted rows",
            "Report row cap",
        ];
        for (offset, label) in evidence_labels.iter().enumerate() {
            sheet
                .write_string_with_format(3 + offset as u32, 0, *label, label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        sheet
            .write_number_with_format(3, 1, evidence.schema_version as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string(4, 1, &excel_safe_text(&evidence.candidate_manifest_hash))
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string(5, 1, &excel_safe_text(&evidence.selection_cohort))
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string(6, 1, &excel_safe_text(&evidence.selection_status))
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string(
                7,
                1,
                if evidence.require_gpu_kernel {
                    "Yes"
                } else {
                    "No"
                },
            )
            .map_err(|e: XlsxError| e.to_string())?;
        for (row, value) in [
            evidence.source_candidate_count,
            evidence.included_candidate_count,
            evidence.omitted_candidate_count,
            evidence.row_cap,
        ]
        .into_iter()
        .enumerate()
        {
            sheet
                .write_number_with_format(8 + row as u32, 1, value as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        let search_labels = [
            "Mode",
            "Search ID",
            "Search schema",
            "Sequence",
            "Seed (exact)",
            "Requested budget",
            "Effective budget",
            "Valid search space",
            "Float identity scale",
            "Halton attempts",
            "Invalid period pairs",
            "Duplicate candidates",
            "Lexicographic fallback (legacy)",
            "Exhaustive candidates",
            "Search config",
        ];
        for (offset, label) in search_labels.iter().enumerate() {
            sheet
                .write_string_with_format(3 + offset as u32, 3, *label, label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        if let Some(search) = evidence.search.as_ref() {
            let mode = if search.sequence == "exhaustive_lexicographic" {
                "Exhaustive lexicographic (sealed manifest)"
            } else {
                "Halton QMC (sealed manifest)"
            };
            sheet
                .write_string(3, 4, mode)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(4, 4, &excel_safe_text(&search.search_id))
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_number_with_format(5, 4, search.schema_version as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(6, 4, &excel_safe_text(&search.sequence))
                .map_err(|e: XlsxError| e.to_string())?;
            // A u64 seed can exceed Excel's exact numeric range. Preserve it
            // as text so replay provenance is never rounded.
            sheet
                .write_string(7, 4, search.seed.to_string())
                .map_err(|e: XlsxError| e.to_string())?;
            for (row, value) in [search.requested_budget, search.effective_budget]
                .into_iter()
                .enumerate()
            {
                sheet
                    .write_number_with_format(8 + row as u32, 4, value as f64, int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
            sheet
                .write_string(10, 4, &excel_safe_text(&search.valid_search_space_size))
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(11, 4, search.float_identity_scale.to_string())
                .map_err(|e: XlsxError| e.to_string())?;
            for (row, value) in [
                search.halton_attempts,
                search.invalid_period_pairs_discarded,
                search.duplicate_candidates_discarded,
                search.lexicographic_fallback_candidates,
                search.exhaustive_candidates,
            ]
            .into_iter()
            .enumerate()
            {
                sheet
                    .write_number_with_format(12 + row as u32, 4, value as f64, int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
            let config = search
                .config
                .as_ref()
                .and_then(|config| serde_json::to_string(config).ok())
                .unwrap_or_else(|| "-".to_string());
            sheet
                .write_string_with_format(17, 4, &excel_safe_text(&config), wrap_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            sheet
                .write_string(3, 4, "Legacy fixed period-pair manifest")
                .map_err(|e: XlsxError| e.to_string())?;
            for row in 4..=17 {
                sheet
                    .write_string(row, 4, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
            }
        }
    } else {
        sheet
            .write_string_with_format(3, 0, "Evidence state", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string(3, 1, "Unavailable; no values inferred")
            .map_err(|e: XlsxError| e.to_string())?;
        for row in 4..=11 {
            sheet
                .write_string(row, 1, "-")
                .map_err(|e: XlsxError| e.to_string())?;
        }
        sheet
            .write_string_with_format(3, 3, "Mode", label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        sheet
            .write_string(3, 4, "Unavailable")
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let policy = lineage.cross_universe_admission;
    let policy_labels = [
        "Minimum profitable share",
        "Minimum hard-gate share",
        "Minimum PnL p10",
        "Minimum Sharpe p10",
        "Admitted candidates",
        "Selected candidate ID",
    ];
    for (offset, label) in policy_labels.iter().enumerate() {
        sheet
            .write_string_with_format(3 + offset as u32, 6, *label, label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    write_finite_number_or_dash(sheet, 3, 7, policy.min_profitable_share_requested, pct_fmt)?;
    write_finite_number_or_dash(
        sheet,
        4,
        7,
        policy.min_hard_gate_pass_share_requested,
        pct_fmt,
    )?;
    write_finite_number_or_dash(sheet, 5, 7, policy.min_pnl_p10, num6_fmt)?;
    write_finite_number_or_dash(sheet, 6, 7, policy.min_sharpe_p10, num6_fmt)?;
    sheet
        .write_number_with_format(7, 7, lineage.admitted_candidate_count as f64, int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .write_string(
            8,
            7,
            &excel_safe_text(lineage.selected_candidate_id.as_deref().unwrap_or("-")),
        )
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(config) = lineage
        .vcet_candidate_evidence
        .as_ref()
        .and_then(|evidence| evidence.search.as_ref())
        .and_then(|search| search.config.as_ref())
    {
        let axis_headers = ["Search axis", "Min / choices", "Max", "Step / quantum"];
        for (offset, header) in axis_headers.iter().enumerate() {
            sheet
                .write_string_with_format(2, 9 + offset as u16, *header, header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        let integer_axes = [
            (3u32, "VIDYA short", &config.vidya_short_period),
            (4, "VIDYA long", &config.vidya_long_period),
            (6, "CE period", &config.ce_period),
            (10, "Max trade bars", &config.max_trade_bars),
        ];
        for (row, label, axis) in integer_axes {
            sheet
                .write_string(row, 9, label)
                .map_err(|e: XlsxError| e.to_string())?;
            for (column, value) in [(10u16, axis.min), (11, axis.max), (12, axis.step)] {
                sheet
                    .write_number_with_format(row, column, f64::from(value), int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
        }
        for (row, label, axis) in [
            (5u32, "VIDYA alpha", &config.vidya_alpha),
            (7, "CE multiplier", &config.ce_mult),
        ] {
            sheet
                .write_string(row, 9, label)
                .map_err(|e: XlsxError| e.to_string())?;
            for (column, value) in [(10u16, axis.min), (11, axis.max), (12, axis.quantum)] {
                write_finite_number_or_dash(sheet, row, column, value, num6_fmt)?;
            }
        }
        for (row, label, choices) in [
            (8u32, "CE use close", &config.ce_use_close.values),
            (9, "Require VIDYA slope", &config.require_vidya_slope.values),
        ] {
            let choices = choices
                .iter()
                .map(|value| if *value { "true" } else { "false" })
                .collect::<Vec<_>>()
                .join(", ");
            sheet
                .write_string(row, 9, label)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(row, 10, &choices)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(row, 11, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(row, 12, "-")
                .map_err(|e: XlsxError| e.to_string())?;
        }
        add_native_table(
            sheet,
            2,
            9,
            10,
            12,
            "VcetSearchAxesTable",
            &axis_headers,
            TableStyle::Medium4,
        )?;
    }

    for (first_col, last_col, label) in [
        (0u16, 4u16, "Identity and selection"),
        (5, 12, "Effective VCET controls"),
        (13, 25, "Training cohort (selection evidence)"),
        (26, 38, "Holdout cohort (not used for selection)"),
        (39, 45, "All requested datasets"),
    ] {
        sheet
            .merge_range(
                VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32 - 1,
                first_col,
                VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32 - 1,
                last_col,
                label,
                header_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let headers: [&str; 46] = [
        "Manifest ordinal (0-based)",
        "Candidate ID",
        "Selected",
        "Admitted",
        "Parameter evidence",
        "VIDYA short",
        "VIDYA long",
        "VIDYA alpha",
        "CE period",
        "CE multiplier",
        "CE use close",
        "Require VIDYA slope",
        "Max trade bars",
        "Train requested",
        "Train evaluated",
        "Train missing/failed",
        "Train profitable",
        "Train hard-gate pass",
        "Train profitable share",
        "Train hard-gate share",
        "Train PnL p10 (native)",
        "Train PnL median (native)",
        "Train Sharpe p10",
        "Train Sharpe median",
        "Train MaxDD p90",
        "Train trades median",
        "Holdout requested",
        "Holdout evaluated",
        "Holdout missing/failed",
        "Holdout profitable",
        "Holdout hard-gate pass",
        "Holdout profitable share",
        "Holdout hard-gate share",
        "Holdout PnL p10 (native)",
        "Holdout PnL median (native)",
        "Holdout Sharpe p10",
        "Holdout Sharpe median",
        "Holdout MaxDD p90",
        "Holdout trades median",
        "All requested",
        "All evaluated",
        "All missing/failed",
        "All profitable share",
        "All hard-gate share",
        "All PnL p10 (native)",
        "All Sharpe p10",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(
                VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32,
                col as u16,
                *header,
                header_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }
    let _ = sheet.set_row_height(VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32, 42.0);

    let selected_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xDCFCE7))
        .set_font_color(Color::RGB(0x166534))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let admitted_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xDBEAFE))
        .set_font_color(Color::RGB(0x1E40AF))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let unavailable_fmt = Format::new()
        .set_background_color(Color::RGB(0xFEF3C7))
        .set_font_color(Color::RGB(0x92400E))
        .set_text_wrap()
        .set_border(FormatBorder::Thin);

    let data_start = VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32 + 1;
    if let Some(evidence) = lineage.vcet_candidate_evidence.as_ref() {
        for (offset, candidate) in evidence.candidates.iter().enumerate() {
            let row = data_start + offset as u32;
            sheet
                .write_number_with_format(row, 0, candidate.manifest_ordinal as f64, int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string(row, 1, &excel_safe_text(&candidate.candidate_id))
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string_with_format(
                    row,
                    2,
                    if candidate.selected { "Yes" } else { "No" },
                    if candidate.selected {
                        &selected_fmt
                    } else {
                        wrap_fmt
                    },
                )
                .map_err(|e: XlsxError| e.to_string())?;
            sheet
                .write_string_with_format(
                    row,
                    3,
                    if candidate.admitted { "Yes" } else { "No" },
                    if candidate.admitted {
                        &admitted_fmt
                    } else {
                        wrap_fmt
                    },
                )
                .map_err(|e: XlsxError| e.to_string())?;
            let controls = &candidate.controls;
            let parameter_format = if controls.evidence_state.starts_with("unavailable")
                || controls.evidence_state.starts_with("invalid")
            {
                &unavailable_fmt
            } else {
                wrap_fmt
            };
            sheet
                .write_string_with_format(
                    row,
                    4,
                    &excel_safe_text(&controls.evidence_state),
                    parameter_format,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            write_optional_integer_or_dash(sheet, row, 5, controls.vidya_short_period, int_fmt)?;
            write_optional_integer_or_dash(sheet, row, 6, controls.vidya_long_period, int_fmt)?;
            write_optional_finite_number_or_dash(sheet, row, 7, controls.vidya_alpha, num6_fmt)?;
            write_optional_integer_or_dash(sheet, row, 8, controls.ce_period, int_fmt)?;
            write_optional_finite_number_or_dash(sheet, row, 9, controls.ce_mult, num6_fmt)?;
            write_optional_bool_or_dash(sheet, row, 10, controls.ce_use_close)?;
            write_optional_bool_or_dash(sheet, row, 11, controls.require_vidya_slope)?;
            write_optional_integer_or_dash(sheet, row, 12, controls.max_trade_bars, int_fmt)?;

            write_vcet_candidate_cohort_metrics(
                sheet,
                row,
                13,
                Some(&candidate.training),
                int_fmt,
                pct_fmt,
                num6_fmt,
                num6_fmt,
            )?;
            write_vcet_candidate_cohort_metrics(
                sheet,
                row,
                26,
                candidate.holdout.as_ref(),
                int_fmt,
                pct_fmt,
                num6_fmt,
                num6_fmt,
            )?;
            if let Some(all) = candidate.all_universe.as_ref() {
                for (column, value) in [
                    (39u16, all.requested_datasets),
                    (40, all.evaluated_datasets),
                    (41, all.missing_or_failed_datasets),
                ] {
                    sheet
                        .write_number_with_format(row, column, value as f64, int_fmt)
                        .map_err(|e: XlsxError| e.to_string())?;
                }
                write_finite_number_or_dash(
                    sheet,
                    row,
                    42,
                    all.profitable_share_requested,
                    pct_fmt,
                )?;
                write_finite_number_or_dash(
                    sheet,
                    row,
                    43,
                    all.hard_gate_pass_share_requested,
                    pct_fmt,
                )?;
                write_optional_finite_number_or_dash(sheet, row, 44, all.pnl_p10, num6_fmt)?;
                write_optional_finite_number_or_dash(sheet, row, 45, all.sharpe_p10, num6_fmt)?;
            } else {
                for column in 39..=45 {
                    sheet
                        .write_string(row, column, "-")
                        .map_err(|e: XlsxError| e.to_string())?;
                }
            }
        }

        if evidence.candidates.is_empty() {
            sheet
                .merge_range(
                    data_start,
                    0,
                    data_start,
                    last_col,
                    "No candidate rows were retained in the bounded evidence projection.",
                    wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            let data_end = data_start + evidence.candidates.len() as u32 - 1;
            add_native_table(
                sheet,
                VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32,
                0,
                data_end,
                last_col,
                "VcetCandidateEvidenceTable",
                &headers,
                TableStyle::Medium2,
            )?;

            let share_heatmap = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, 0.0)
                .set_midpoint(ConditionalFormatType::Number, 0.5)
                .set_maximum(ConditionalFormatType::Number, 1.0)
                .set_minimum_color("F8696B")
                .set_midpoint_color("FFEB84")
                .set_maximum_color("63BE7B");
            for (first_col, last_col) in [(18u16, 19u16), (31, 32), (42, 43)] {
                sheet
                    .add_conditional_format(
                        data_start,
                        first_col,
                        data_end,
                        last_col,
                        &share_heatmap,
                    )
                    .map_err(|e: XlsxError| e.to_string())?;
            }

            let pnl_bound = evidence
                .candidates
                .iter()
                .flat_map(|candidate| {
                    [
                        candidate.training.pnl_p10,
                        candidate
                            .holdout
                            .as_ref()
                            .and_then(|aggregate| aggregate.pnl_p10),
                        candidate
                            .all_universe
                            .as_ref()
                            .and_then(|aggregate| aggregate.pnl_p10),
                    ]
                })
                .flatten()
                .filter(|value| value.is_finite())
                .map(f64::abs)
                .fold(0.0f64, f64::max)
                .max(1e-9);
            let pnl_heatmap = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, -pnl_bound)
                .set_midpoint(ConditionalFormatType::Number, 0.0)
                .set_maximum(ConditionalFormatType::Number, pnl_bound)
                .set_minimum_color("F8696B")
                .set_midpoint_color("FFEB84")
                .set_maximum_color("63BE7B");
            for column in [20u16, 33, 44] {
                sheet
                    .add_conditional_format(data_start, column, data_end, column, &pnl_heatmap)
                    .map_err(|e: XlsxError| e.to_string())?;
            }

            let sharpe_bound = evidence
                .candidates
                .iter()
                .flat_map(|candidate| {
                    [
                        candidate.training.sharpe_p10,
                        candidate
                            .holdout
                            .as_ref()
                            .and_then(|aggregate| aggregate.sharpe_p10),
                        candidate
                            .all_universe
                            .as_ref()
                            .and_then(|aggregate| aggregate.sharpe_p10),
                    ]
                })
                .flatten()
                .filter(|value| value.is_finite())
                .map(f64::abs)
                .fold(0.0f64, f64::max)
                .max(1e-9);
            let sharpe_heatmap = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, -sharpe_bound)
                .set_midpoint(ConditionalFormatType::Number, 0.0)
                .set_maximum(ConditionalFormatType::Number, sharpe_bound)
                .set_minimum_color("F8696B")
                .set_midpoint_color("FFEB84")
                .set_maximum_color("63BE7B");
            for column in [22u16, 35, 45] {
                sheet
                    .add_conditional_format(data_start, column, data_end, column, &sharpe_heatmap)
                    .map_err(|e: XlsxError| e.to_string())?;
            }

            let dd_bound = evidence
                .candidates
                .iter()
                .flat_map(|candidate| {
                    [
                        candidate.training.max_drawdown_p90,
                        candidate
                            .holdout
                            .as_ref()
                            .and_then(|aggregate| aggregate.max_drawdown_p90),
                    ]
                })
                .flatten()
                .filter(|value| value.is_finite())
                .fold(0.0f64, f64::max)
                .max(1e-9);
            let drawdown_heatmap = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, 0.0)
                .set_midpoint(ConditionalFormatType::Number, dd_bound / 2.0)
                .set_maximum(ConditionalFormatType::Number, dd_bound)
                .set_minimum_color("63BE7B")
                .set_midpoint_color("FFEB84")
                .set_maximum_color("F8696B");
            for column in [24u16, 37] {
                sheet
                    .add_conditional_format(data_start, column, data_end, column, &drawdown_heatmap)
                    .map_err(|e: XlsxError| e.to_string())?;
            }

            let chart_rows = evidence.candidates.len().min(VCET_CANDIDATE_CHART_ROW_CAP);
            let has_chart_data = evidence
                .candidates
                .iter()
                .take(chart_rows)
                .any(|candidate| {
                    candidate.training.profitable_share_requested.is_finite()
                        && candidate.training.sharpe_p10.is_some_and(f64::is_finite)
                });
            if has_chart_data {
                let chart_end = data_start + chart_rows as u32 - 1;
                let mut marker = ChartMarker::new();
                marker
                    .set_type(ChartMarkerType::Circle)
                    .set_size(7)
                    .set_format(
                        ChartFormat::new()
                            .set_solid_fill(ChartSolidFill::new().set_color("#0F766E")),
                    );
                let mut chart = Chart::new(ChartType::Scatter);
                chart
                    .add_series()
                    .set_name("Training candidates")
                    .set_categories(("VCET Candidate Evidence", data_start, 18, chart_end, 18))
                    .set_values(("VCET Candidate Evidence", data_start, 22, chart_end, 22))
                    .set_marker(&marker);
                chart.title().set_name(
                    if evidence.candidates.len() > VCET_CANDIDATE_CHART_ROW_CAP {
                        "Training profitability share vs Sharpe p10 (first 100 evidence rows)"
                    } else {
                        "Training profitability share vs Sharpe p10"
                    },
                );
                chart
                    .x_axis()
                    .set_name("Profitable share of requested training datasets")
                    .set_num_format("0.0%");
                chart
                    .y_axis()
                    .set_name("Training Sharpe p10")
                    .set_num_format("0.000");
                chart.legend().set_hidden();
                chart.set_width(600).set_height(320);
                // Keep the chart inside the sheet's existing evidence-table
                // width. Rows 2..18 to the right of the summary cards are an
                // intentional dashboard area; placing the chart beyond column
                // AT made the rendered/printed canvas needlessly enormous.
                sheet
                    .insert_chart_with_offset(2, 14, &chart, 10, 10)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
        }
    } else {
        sheet
            .merge_range(
                data_start,
                0,
                data_start,
                last_col,
                "No sealed VCET candidate evidence is available; all candidate fields are intentionally blank rather than inferred.",
                wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let widths: [f64; 46] = [
        14.0, 68.0, 10.0, 10.0, 34.0, 12.0, 12.0, 12.0, 11.0, 13.0, 13.0, 18.0, 14.0, 13.0, 13.0,
        16.0, 14.0, 17.0, 17.0, 17.0, 14.0, 15.0, 14.0, 15.0, 14.0, 15.0, 15.0, 15.0, 18.0, 16.0,
        18.0, 19.0, 18.0, 16.0, 17.0, 16.0, 17.0, 16.0, 17.0, 13.0, 13.0, 16.0, 17.0, 17.0, 14.0,
        15.0,
    ];
    for (column, width) in widths.iter().enumerate() {
        let _ = sheet.set_column_width(column as u16, *width);
    }
    // The summary blocks share A/B, D/E, and G/H with the detailed table.
    // Keep their long provenance values readable at normal zoom.
    let _ = sheet.set_column_width(1, 68.0);
    let _ = sheet.set_column_width(4, 68.0);
    let _ = sheet.set_column_width(7, 24.0);
    sheet
        .set_freeze_panes(data_start, 2)
        .map_err(|e: XlsxError| e.to_string())?;
    sheet
        .set_repeat_rows(
            VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32 - 1,
            VCET_CANDIDATE_EVIDENCE_HEADER_ROW as u32,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    Ok(())
}

pub(crate) fn build_meta_report_xlsx_bytes(
    ts: &str,
    meta: &MetaSelectionResult,
    holdout_execution: Option<&MetaHoldoutExecutionEvidence>,
    cohort_manifest: Option<&MetaCohortManifest>,
) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};

    validate_meta_xlsx_limits(meta, holdout_execution, cohort_manifest)?;
    let matrix_collision_labels = matrix_dataset_collision_labels(&meta.groups);
    let holdout_collision_labels = holdout_dataset_collision_labels(&meta.groups);
    let mut workbook = Workbook::new();

    let title_fmt = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);
    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0B1220))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let label_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF8FAFC))
        .set_border(FormatBorder::Thin);
    let wrap_fmt = Format::new()
        .set_text_wrap()
        .set_align(FormatAlign::Top)
        .set_border(FormatBorder::Thin);
    let int_fmt = Format::new().set_num_format("0");
    let num6_fmt = Format::new().set_num_format("0.000000");
    let pct_fmt = Format::new().set_num_format("0.0%");
    let pnl_pct_fmt = Format::new().set_num_format("0.00%;[Red](0.00%);-");

    {
        let summary = workbook.add_worksheet();
        summary
            .set_name("Meta Analysis")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(summary, "VectorGrid Meta Analysis");

        summary
            .merge_range(0, 0, 0, 21, "VectorGrid Stage 3 Meta Analysis", &title_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(1, 0, 1, 1, "Timestamp", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_string_with_format(1, 2, &excel_safe_text(ts), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(1, 3, 1, 4, "Strategy", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_string_with_format(1, 5, &excel_safe_text(&meta.strategy_kind), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(1, 6, 1, 7, "Timeframe", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_string_with_format(
                1,
                8,
                &excel_safe_text(meta.timeframe.as_deref().unwrap_or("")),
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(2, 0, 2, 1, "Artifacts considered", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(2, 2, meta.artifacts_considered as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(2, 3, 2, 4, "Artifacts used", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(2, 5, meta.artifacts_used as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(2, 6, 2, 7, "Candidates considered", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(2, 8, meta.candidates_considered as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(3, 0, 3, 1, "Runtime (ms)", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(3, 2, meta.runtime_ms as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        if holdout_execution.is_some() || meta.holdout.is_some() {
            let datasets_requested = holdout_execution
                .map(|execution| execution.datasets_requested)
                .or_else(|| {
                    meta.holdout
                        .as_ref()
                        .map(|holdout| holdout.datasets.saturating_add(holdout.skipped))
                })
                .unwrap_or_default();
            let datasets_evaluated = holdout_execution
                .map(|execution| execution.datasets_evaluated)
                .or_else(|| meta.holdout.as_ref().map(|holdout| holdout.datasets))
                .unwrap_or_default();
            let datasets_failed = holdout_execution
                .map(|execution| execution.datasets_failed)
                .or_else(|| meta.holdout.as_ref().map(|holdout| holdout.skipped))
                .unwrap_or_default();

            summary
                .merge_range(3, 3, 3, 4, "Holdout requested", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(3, 5, datasets_requested as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(3, 6, 3, 7, "Holdout evaluated", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(3, 8, datasets_evaluated as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(3, 9, 3, 10, "Holdout failed", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(3, 11, datasets_failed as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if let Some(execution) = holdout_execution {
            summary
                .merge_range(4, 0, 4, 1, "Holdout backend", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string_with_format(
                    4,
                    2,
                    &excel_safe_text(&execution.requested_backend),
                    &wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(4, 3, 4, 4, "GPU required", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string_with_format(
                    4,
                    5,
                    if execution.require_gpu { "Yes" } else { "No" },
                    &wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(4, 6, 4, 7, "Fully GPU", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(4, 8, execution.gpu_datasets as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(4, 9, 4, 10, "Fully CPU", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(4, 11, execution.cpu_datasets as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(4, 12, 4, 13, "Hybrid", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(4, 14, execution.hybrid_datasets as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(4, 15, 4, 16, "Fallback", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(4, 17, execution.fallback_datasets as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(4, 18, 4, 19, "GPU device", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            match execution.gpu_device_id {
                Some(device_id) => summary
                    .write_number_with_format(4, 20, device_id as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?,
                None => summary
                    .write_string_with_format(4, 20, "-", &wrap_fmt)
                    .map_err(|e: XlsxError| e.to_string())?,
            };
        }

        if !meta.warnings.is_empty() {
            let warnings_text = excel_safe_text(&meta.warnings.join(" | "));
            summary
                .merge_range(5, 0, 5, 1, "Warnings", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .merge_range(5, 2, 5, 21, &warnings_text, &wrap_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            let wrapped_lines = warnings_text.chars().count().saturating_sub(1) / 120 + 1;
            let warning_height = (24.0 + wrapped_lines.min(6) as f64 * 14.0).min(120.0);
            let _ = summary.set_row_height(5, warning_height);
        }

        let header_row: u32 = 6;
        let headers: [&str; 22] = [
            "Rank",
            "Samples",
            "Datasets",
            "Pass rate",
            "Meta score",
            "Final score",
            "Robust p10",
            "Robust med",
            "Tail gap",
            "MaxDD p90",
            "Cross med",
            "PSR med",
            "DSR med",
            "H pass",
            "H score p10",
            "H score med",
            "H DD p90",
            "H Sharpe med",
            "H PnL med",
            "Train rows",
            "MA source",
            "Params",
        ];
        for (c, h) in headers.iter().enumerate() {
            summary
                .write_string_with_format(header_row, c as u16, *h, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        summary
            .set_freeze_panes(header_row + 1, 1)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .set_repeat_rows(header_row, header_row)
            .map_err(|e: XlsxError| e.to_string())?;

        let widths: [f64; 22] = [
            10.0, 10.0, 22.0, 9.0, 11.0, 11.0, 10.0, 10.0, 10.0, 10.0, 10.0, 9.0, 9.0, 9.0, 10.0,
            10.0, 10.0, 11.0, 11.0, 9.0, 10.0, 80.0,
        ];
        for (c, w) in widths.iter().enumerate() {
            let _ = summary.set_column_width(c as u16, *w);
        }

        for (i, row) in meta.groups.iter().enumerate() {
            let r = header_row + 1 + i as u32;
            let final_score = row.final_score.unwrap_or(row.meta_score);
            let params_json =
                excel_safe_text(&serde_json::to_string(&row.params).unwrap_or_default());

            summary
                .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 1, row.samples as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 2, row.datasets_covered as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            write_finite_number_or_dash(summary, r, 3, row.pass_rate, &pct_fmt)?;
            write_finite_number_or_dash(summary, r, 4, row.meta_score, &num6_fmt)?;
            write_finite_number_or_dash(summary, r, 5, final_score, &num6_fmt)?;
            write_finite_number_or_dash(summary, r, 6, row.robust_score_p10, &num6_fmt)?;
            write_finite_number_or_dash(summary, r, 7, row.robust_score_median, &num6_fmt)?;
            write_finite_number_or_dash(summary, r, 8, row.tail_gap, &num6_fmt)?;
            write_finite_number_or_dash(summary, r, 9, row.max_dd_p90, &pct_fmt)?;
            if let Some(cross_score_median) = row.cross_score_median {
                write_finite_number_or_dash(summary, r, 10, cross_score_median, &num6_fmt)?;
            } else {
                summary
                    .write_string(r, 10, "")
                    .map_err(|e: XlsxError| e.to_string())?;
            }
            write_finite_number_or_dash(summary, r, 11, row.psr_median, &pct_fmt)?;
            write_finite_number_or_dash(summary, r, 12, row.dsr_median, &pct_fmt)?;

            if let Some(h) = row.holdout.as_ref() {
                write_finite_number_or_dash(summary, r, 13, h.pass_rate, &pct_fmt)?;
                write_finite_number_or_dash(summary, r, 14, h.score_p10, &num6_fmt)?;
                write_finite_number_or_dash(summary, r, 15, h.score_median, &num6_fmt)?;
                write_finite_number_or_dash(summary, r, 16, h.max_dd_p90, &pct_fmt)?;
                write_finite_number_or_dash(summary, r, 17, h.sharpe_median, &num6_fmt)?;
                write_finite_number_or_dash(summary, r, 18, h.pnl_median, &num6_fmt)?;
            } else {
                summary
                    .write_string(r, 13, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_string(r, 14, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_string(r, 15, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_string(r, 16, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_string(r, 17, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_string(r, 18, "-")
                    .map_err(|e: XlsxError| e.to_string())?;
            }

            summary
                .write_number_with_format(r, 19, row.train_rows.len() as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string(r, 20, &excel_safe_text(&row.ma_source))
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string(r, 21, &params_json)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if !meta.groups.is_empty() {
            let row_min = header_row + 1;
            let row_max = header_row + meta.groups.len() as u32;
            let chart_row_count = meta.groups.len().min(SUMMARY_CHART_ROW_CAP);
            let chart_row_max = header_row + chart_row_count as u32;
            add_native_table(
                summary,
                header_row,
                0,
                row_max,
                headers.len().saturating_sub(1) as u16,
                "MetaAnalysisTable",
                &headers,
                TableStyle::Medium2,
            )?;

            let mut final_chart = Chart::new(ChartType::Column);
            final_chart
                .add_series()
                .set_categories(("Meta Analysis", row_min, 0, chart_row_max, 0))
                .set_values(("Meta Analysis", row_min, 5, chart_row_max, 5))
                .set_format(
                    ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#2563EB")),
                );
            final_chart
                .title()
                .set_name(if meta.groups.len() > SUMMARY_CHART_ROW_CAP {
                    "Final score by rank (top 100)"
                } else {
                    "Final score by rank"
                });
            final_chart.x_axis().set_name("Rank");
            final_chart
                .x_axis()
                .set_num_format("0")
                .set_major_gridlines(false);
            final_chart
                .y_axis()
                .set_name("Final score")
                .set_num_format("0.000");
            final_chart.legend().set_hidden();
            final_chart.set_width(500).set_height(300);
            summary
                .insert_chart_with_offset(1, 24, &final_chart, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;

            let mut scatter_marker = ChartMarker::new();
            scatter_marker
                .set_type(ChartMarkerType::Circle)
                .set_size(7)
                .set_format(
                    ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#0F766E")),
                );
            let mut scatter = Chart::new(ChartType::Scatter);
            scatter
                .add_series()
                .set_name("Pass vs robust")
                .set_categories(("Meta Analysis", row_min, 3, chart_row_max, 3))
                .set_values(("Meta Analysis", row_min, 6, chart_row_max, 6))
                .set_marker(&scatter_marker);
            scatter
                .title()
                .set_name(if meta.groups.len() > SUMMARY_CHART_ROW_CAP {
                    "Pass rate vs robust p10 (top 100)"
                } else {
                    "Pass rate vs robust p10"
                });
            scatter
                .x_axis()
                .set_name("Pass rate")
                .set_num_format("0.0%");
            scatter
                .y_axis()
                .set_name("Robust p10")
                .set_num_format("0.000");
            scatter.legend().set_hidden();
            scatter.set_width(500).set_height(300);
            summary
                .insert_chart_with_offset(18, 24, &scatter, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    {
        let by_asset = workbook.add_worksheet();
        by_asset
            .set_name("By Asset (Train)")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(by_asset, "VectorGrid Meta Analysis - Training Assets");
        by_asset
            .merge_range(
                0,
                0,
                0,
                15,
                "Per-asset training rows by meta group",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let headers: [&str; 16] = [
            "Group rank",
            "Matrix dataset",
            "Dataset key",
            "Robust score",
            "PnL",
            "PnL %",
            "Sharpe",
            "MaxDD",
            "Trades",
            "Exposure",
            "Cross score",
            "Cross pnl+ frac",
            "PSR",
            "DSR",
            "Artifact ts",
            "Params",
        ];
        let header_row: u32 = 2;
        for (c, h) in headers.iter().enumerate() {
            by_asset
                .write_string_with_format(header_row, c as u16, *h, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        by_asset
            .set_freeze_panes(header_row + 1, 1)
            .map_err(|e: XlsxError| e.to_string())?;
        by_asset
            .set_repeat_rows(header_row, header_row)
            .map_err(|e: XlsxError| e.to_string())?;
        let widths: [f64; 16] = [
            10.0, 36.0, 28.0, 11.0, 10.0, 10.0, 9.0, 9.0, 8.0, 8.0, 10.0, 12.0, 8.0, 8.0, 24.0,
            70.0,
        ];
        for (c, w) in widths.iter().enumerate() {
            let _ = by_asset.set_column_width(c as u16, *w);
        }

        let mut row_ix = header_row + 1;
        for (i, g) in meta.groups.iter().enumerate() {
            let params_json =
                excel_safe_text(&serde_json::to_string(&g.params).unwrap_or_default());
            for tr in &g.train_rows {
                by_asset
                    .write_number_with_format(row_ix, 0, (i + 1) as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_string(
                        row_ix,
                        1,
                        &excel_safe_text(&matrix_dataset_label(tr, &matrix_collision_labels)),
                    )
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_string(
                        row_ix,
                        2,
                        &excel_safe_text(&share_safe_dataset_key(&tr.dataset_key)),
                    )
                    .map_err(|e: XlsxError| e.to_string())?;
                write_finite_number_or_dash(by_asset, row_ix, 3, tr.robust_score, &num6_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 4, tr.pnl, &num6_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 5, tr.pnl_pct, &pnl_pct_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 6, tr.sharpe, &num6_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 7, tr.max_dd, &pct_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 8, tr.trades, &int_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 9, tr.exposure, &pct_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 10, tr.cross_score, &num6_fmt)?;
                write_finite_number_or_dash(
                    by_asset,
                    row_ix,
                    11,
                    tr.cross_pnl_positive_frac,
                    &pct_fmt,
                )?;
                write_finite_number_or_dash(by_asset, row_ix, 12, tr.psr, &pct_fmt)?;
                write_finite_number_or_dash(by_asset, row_ix, 13, tr.dsr, &pct_fmt)?;
                by_asset
                    .write_string(row_ix, 14, &excel_safe_text(tr.ts.as_deref().unwrap_or("")))
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_string(row_ix, 15, &params_json)
                    .map_err(|e: XlsxError| e.to_string())?;
                row_ix += 1;
            }
        }
        if row_ix == header_row + 1 {
            by_asset
                .write_string(
                    row_ix,
                    0,
                    "No training rows were captured for this meta run.",
                )
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            add_native_table(
                by_asset,
                header_row,
                0,
                row_ix - 1,
                headers.len().saturating_sub(1) as u16,
                "ByAssetTrainTable",
                &headers,
                TableStyle::Medium4,
            )?;
        }
    }

    {
        let matrix = workbook.add_worksheet();
        matrix
            .set_name("Asset Matrix")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(matrix, "VectorGrid Meta Analysis - Asset Matrix");

        let mut datasets: BTreeSet<String> = BTreeSet::new();
        for g in &meta.groups {
            for tr in &g.train_rows {
                datasets.insert(matrix_dataset_label(tr, &matrix_collision_labels));
            }
        }
        validate_matrix_dataset_headers(&datasets)?;
        let matrix_last_col = asset_matrix_last_column(datasets.len())?;
        matrix
            .merge_range(
                0,
                0,
                0,
                matrix_last_col.max(10),
                "Per-asset PnL% matrix (median by dataset)",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        if datasets.is_empty() {
            matrix
                .write_string(2, 0, "No training rows available.")
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            let matrix_header_fmt = header_fmt.clone().set_text_wrap();
            let _ = matrix.set_row_height(2, 84.0);
            matrix
                .write_string_with_format(2, 0, "Rank", &matrix_header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            matrix
                .write_string_with_format(2, 1, "Meta score", &matrix_header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            matrix
                .write_string_with_format(2, 2, "Final score", &matrix_header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;

            for (i, ds) in datasets.iter().enumerate() {
                matrix
                    .write_string_with_format(2, (3 + i) as u16, ds, &matrix_header_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                let _ = matrix.set_column_width((3 + i) as u16, 18.0);
            }
            let _ = matrix.set_column_width(0, 6.0);
            let _ = matrix.set_column_width(1, 11.0);
            let _ = matrix.set_column_width(2, 11.0);
            matrix
                .set_freeze_panes(3, 3)
                .map_err(|e: XlsxError| e.to_string())?;
            matrix
                .set_repeat_rows(2, 2)
                .map_err(|e: XlsxError| e.to_string())?;

            let mut heatmap_max_abs = 0.0_f64;
            for (i, g) in meta.groups.iter().enumerate() {
                let r = 3 + i as u32;
                let final_score = g.final_score.unwrap_or(g.meta_score);
                let by_ds =
                    median_train_pnl_pct_by_dataset(&g.train_rows, &matrix_collision_labels);

                matrix
                    .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                write_finite_number_or_dash(matrix, r, 1, g.meta_score, &num6_fmt)?;
                write_finite_number_or_dash(matrix, r, 2, final_score, &num6_fmt)?;

                for (j, ds) in datasets.iter().enumerate() {
                    if let Some(v) = by_ds.get(ds) {
                        heatmap_max_abs = heatmap_max_abs.max(v.abs());
                        write_finite_number_or_dash(matrix, r, (3 + j) as u16, *v, &pnl_pct_fmt)?;
                    } else {
                        matrix
                            .write_string(r, (3 + j) as u16, "-")
                            .map_err(|e: XlsxError| e.to_string())?;
                    }
                }
            }

            let row_max = 3 + meta.groups.len().saturating_sub(1) as u32;
            let heatmap_bound = if heatmap_max_abs > 0.0 {
                heatmap_max_abs
            } else {
                1.0
            };
            let pnl_heatmap = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, -heatmap_bound)
                .set_midpoint(ConditionalFormatType::Number, 0.0)
                .set_maximum(ConditionalFormatType::Number, heatmap_bound)
                .set_minimum_color("F8696B")
                .set_midpoint_color("FFEB84")
                .set_maximum_color("63BE7B");
            matrix
                .add_conditional_format(3, 3, row_max, matrix_last_col, &pnl_heatmap)
                .map_err(|e: XlsxError| e.to_string())?;

            let mut table_headers = Vec::with_capacity(3usize.saturating_add(datasets.len()));
            table_headers.extend(["Rank", "Meta score", "Final score"]);
            table_headers.extend(datasets.iter().map(String::as_str));
            add_native_table(
                matrix,
                2,
                0,
                row_max,
                matrix_last_col,
                "AssetMatrixTable",
                &table_headers,
                TableStyle::Medium9,
            )?;
        }
    }

    {
        let holdout = workbook.add_worksheet();
        holdout
            .set_name("Holdout Rows")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(holdout, "VectorGrid Meta Analysis - Holdout Rows");
        holdout
            .merge_range(
                0,
                0,
                0,
                9,
                "Per-asset holdout rows by meta group",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let headers: [&str; 10] = [
            "Group rank",
            "Dataset identity",
            "Dataset key",
            "Pass",
            "Score med",
            "Score p10",
            "Sharpe med",
            "MaxDD worst",
            "PnL med",
            "Group params",
        ];
        let header_row: u32 = 2;
        for (c, h) in headers.iter().enumerate() {
            holdout
                .write_string_with_format(header_row, c as u16, *h, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        holdout
            .set_freeze_panes(header_row + 1, 1)
            .map_err(|e: XlsxError| e.to_string())?;
        holdout
            .set_repeat_rows(header_row, header_row)
            .map_err(|e: XlsxError| e.to_string())?;
        let widths: [f64; 10] = [10.0, 36.0, 28.0, 8.0, 10.0, 10.0, 10.0, 11.0, 10.0, 70.0];
        for (c, w) in widths.iter().enumerate() {
            let _ = holdout.set_column_width(c as u16, *w);
        }

        let mut row_ix = header_row + 1;
        for (i, g) in meta.groups.iter().enumerate() {
            let params_json =
                excel_safe_text(&serde_json::to_string(&g.params).unwrap_or_default());
            for hr in &g.holdout_rows {
                holdout
                    .write_number_with_format(row_ix, 0, (i + 1) as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_string(
                        row_ix,
                        1,
                        &excel_safe_text(&holdout_dataset_label(hr, &holdout_collision_labels)),
                    )
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_string(row_ix, 2, &excel_safe_text(&hr.dataset_key))
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 3, if hr.pass { 1.0 } else { 0.0 }, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                write_finite_number_or_dash(holdout, row_ix, 4, hr.score_median, &num6_fmt)?;
                write_finite_number_or_dash(holdout, row_ix, 5, hr.score_p10, &num6_fmt)?;
                write_finite_number_or_dash(holdout, row_ix, 6, hr.sharpe_median, &num6_fmt)?;
                write_finite_number_or_dash(holdout, row_ix, 7, hr.max_dd_worst, &pct_fmt)?;
                write_finite_number_or_dash(holdout, row_ix, 8, hr.pnl_median, &num6_fmt)?;
                holdout
                    .write_string(row_ix, 9, &params_json)
                    .map_err(|e: XlsxError| e.to_string())?;
                row_ix += 1;
            }
        }
        if row_ix == header_row + 1 {
            holdout
                .write_string(row_ix, 0, "No holdout rows were saved for this run.")
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            add_native_table(
                holdout,
                header_row,
                0,
                row_ix - 1,
                headers.len().saturating_sub(1) as u16,
                "HoldoutRowsTable",
                &headers,
                TableStyle::Medium3,
            )?;
        }
    }

    if let Some(execution) = holdout_execution {
        let configured_auxiliary_stages = if execution.configured_auxiliary_stages.is_empty() {
            "None".to_string()
        } else {
            execution.configured_auxiliary_stages.join(", ")
        };
        let group_header_row =
            holdout_execution_group_header_row(execution.route_validation_batch_counts.len())?
                as u32;
        let group_data_row = group_header_row + 1;
        let execution_sheet = workbook.add_worksheet();
        execution_sheet
            .set_name("Holdout Execution")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(
            execution_sheet,
            "VectorGrid Meta Analysis - Holdout Execution",
        );
        execution_sheet
            .merge_range(
                0,
                0,
                0,
                9,
                "Holdout execution, residency, and requested-universe coverage",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        execution_sheet
            .merge_range(
                1,
                0,
                1,
                9,
                "Evaluation, pass, and profitable rates use requested unique holdout datasets; failed datasets remain in the denominator.",
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let run_headers = ["Run-level metric", "Value"];
        for (col, header) in run_headers.iter().enumerate() {
            execution_sheet
                .write_string_with_format(2, col as u16, *header, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        let run_labels = [
            "Requested backend",
            "GPU required",
            "GPU device",
            "Datasets requested",
            "Datasets evaluated",
            "Datasets failed",
            "Fully GPU datasets",
            "Fully CPU datasets",
            "Hybrid datasets",
            "Fallback datasets",
            "Configured auxiliary stages",
            "Pass-rate denominator",
            "Profitable-rate denominator",
        ];
        for (index, label) in run_labels.iter().enumerate() {
            execution_sheet
                .write_string(3 + index as u32, 0, *label)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        execution_sheet
            .write_string(3, 1, &excel_safe_text(&execution.requested_backend))
            .map_err(|e: XlsxError| e.to_string())?;
        execution_sheet
            .write_string(4, 1, if execution.require_gpu { "Yes" } else { "No" })
            .map_err(|e: XlsxError| e.to_string())?;
        match execution.gpu_device_id {
            Some(device_id) => execution_sheet
                .write_number_with_format(5, 1, device_id as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?,
            None => execution_sheet
                .write_string(5, 1, "-")
                .map_err(|e: XlsxError| e.to_string())?,
        };
        for (row, value) in [
            execution.datasets_requested,
            execution.datasets_evaluated,
            execution.datasets_failed,
            execution.gpu_datasets,
            execution.cpu_datasets,
            execution.hybrid_datasets,
            execution.fallback_datasets,
        ]
        .into_iter()
        .enumerate()
        {
            execution_sheet
                .write_number_with_format(6 + row as u32, 1, value as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        execution_sheet
            .write_string(13, 1, &excel_safe_text(&configured_auxiliary_stages))
            .map_err(|e: XlsxError| e.to_string())?;
        execution_sheet
            .write_string(14, 1, &excel_safe_text(&execution.pass_rate_denominator))
            .map_err(|e: XlsxError| e.to_string())?;
        execution_sheet
            .write_string(
                15,
                1,
                &excel_safe_text(&execution.profitable_rate_denominator),
            )
            .map_err(|e: XlsxError| e.to_string())?;
        add_native_table(
            execution_sheet,
            2,
            0,
            15,
            1,
            "HoldoutExecutionSummaryTable",
            &run_headers,
            TableStyle::Medium2,
        )?;

        let route_headers = ["Observed execution route", "Validation batches"];
        for (offset, header) in route_headers.iter().enumerate() {
            execution_sheet
                .write_string_with_format(2, 4 + offset as u16, *header, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        if execution.route_validation_batch_counts.is_empty() {
            execution_sheet
                .write_string(3, 4, "No route evidence captured.")
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            for (index, (route, count)) in
                execution.route_validation_batch_counts.iter().enumerate()
            {
                let row = 3 + index as u32;
                execution_sheet
                    .write_string(row, 4, &excel_safe_text(route))
                    .map_err(|e: XlsxError| e.to_string())?;
                execution_sheet
                    .write_number_with_format(row, 5, *count as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
            add_native_table(
                execution_sheet,
                2,
                4,
                2 + execution.route_validation_batch_counts.len() as u32,
                5,
                "HoldoutRouteCountsTable",
                &route_headers,
                TableStyle::Medium4,
            )?;
        }

        let group_headers: [&str; 10] = [
            "Final rank",
            "Group key",
            "Requested",
            "Evaluated",
            "Failed",
            "Passes",
            "Profitable",
            "Evaluation rate",
            "Pass rate",
            "Profitable rate",
        ];
        for (col, header) in group_headers.iter().enumerate() {
            execution_sheet
                .write_string_with_format(group_header_row, col as u16, *header, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        let rank_by_key: BTreeMap<&str, usize> = meta
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| (group.key.as_str(), index + 1))
            .collect();
        let mut group_metrics: Vec<_> = execution.group_metrics.iter().collect();
        group_metrics.sort_by_key(|metric| {
            rank_by_key
                .get(metric.group_key.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });

        if group_metrics.is_empty() {
            execution_sheet
                .write_string(
                    group_data_row,
                    0,
                    "No group-level holdout execution evidence captured.",
                )
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            for (index, metric) in group_metrics.iter().enumerate() {
                let row = group_data_row + index as u32;
                if let Some(rank) = rank_by_key.get(metric.group_key.as_str()) {
                    execution_sheet
                        .write_number_with_format(row, 0, *rank as f64, &int_fmt)
                        .map_err(|e: XlsxError| e.to_string())?;
                } else {
                    execution_sheet
                        .write_string(row, 0, "-")
                        .map_err(|e: XlsxError| e.to_string())?;
                }
                execution_sheet
                    .write_string(row, 1, &excel_safe_text(&metric.group_key))
                    .map_err(|e: XlsxError| e.to_string())?;
                for (offset, value) in [
                    metric.datasets_requested,
                    metric.datasets_evaluated,
                    metric.datasets_failed,
                    metric.passes,
                    metric.profitable,
                ]
                .into_iter()
                .enumerate()
                {
                    execution_sheet
                        .write_number_with_format(row, 2 + offset as u16, value as f64, &int_fmt)
                        .map_err(|e: XlsxError| e.to_string())?;
                }
                write_finite_number_or_dash(
                    execution_sheet,
                    row,
                    7,
                    metric.evaluation_rate,
                    &pct_fmt,
                )?;
                write_finite_number_or_dash(execution_sheet, row, 8, metric.pass_rate, &pct_fmt)?;
                write_finite_number_or_dash(
                    execution_sheet,
                    row,
                    9,
                    metric.profitable_rate,
                    &pct_fmt,
                )?;
            }
            let group_last_row = group_header_row + group_metrics.len() as u32;
            add_native_table(
                execution_sheet,
                group_header_row,
                0,
                group_last_row,
                9,
                "HoldoutGroupCoverageTable",
                &group_headers,
                TableStyle::Medium9,
            )?;
            let coverage_heatmap = ConditionalFormat3ColorScale::new()
                .set_minimum(ConditionalFormatType::Number, 0.0)
                .set_midpoint(ConditionalFormatType::Number, 0.5)
                .set_maximum(ConditionalFormatType::Number, 1.0)
                .set_minimum_color("F8696B")
                .set_midpoint_color("FFEB84")
                .set_maximum_color("63BE7B");
            execution_sheet
                .add_conditional_format(group_data_row, 7, group_last_row, 9, &coverage_heatmap)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        // Columns A/B serve the run-level label/value block as well as the
        // group table. Keep the labels readable without making the group
        // table's shared route/failed column excessively wide.
        let execution_widths: [f64; 10] =
            [28.0, 40.0, 12.0, 12.0, 26.0, 12.0, 12.0, 14.0, 12.0, 14.0];
        for (col, width) in execution_widths.iter().enumerate() {
            let _ = execution_sheet.set_column_width(col as u16, *width);
        }
        execution_sheet
            .set_freeze_panes(group_data_row, 2)
            .map_err(|e: XlsxError| e.to_string())?;
        execution_sheet
            .set_repeat_rows(group_header_row, group_header_row)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if let Some(manifest) = cohort_manifest {
        let provenance = workbook.add_worksheet();
        provenance
            .set_name("Cohort Provenance")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(provenance, "VectorGrid Meta Analysis - Cohort Provenance");
        provenance
            .merge_range(
                0,
                0,
                0,
                8,
                "Cohort provenance, disjointness, and campaign lineage",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
        provenance
            .merge_range(
                1,
                0,
                1,
                8,
                "This sheet records the exact training and holdout artifacts used by Meta selection and their campaign-bound universe, when available.",
                &wrap_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let summary_headers = ["Cohort metric", "Value"];
        for (col, header) in summary_headers.iter().enumerate() {
            provenance
                .write_string_with_format(2, col as u16, *header, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        let cohort_labels = [
            "Holdout tag",
            "Disjoint",
            "Train artifacts",
            "Holdout artifacts",
            "Overlapping dataset keys",
            "Missing train dataset keys",
            "Missing holdout dataset keys",
            "Warnings",
        ];
        for (index, label) in cohort_labels.iter().enumerate() {
            provenance
                .write_string(3 + index as u32, 0, *label)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        provenance
            .write_string(
                3,
                1,
                &excel_safe_text(manifest.holdout_tag.as_deref().unwrap_or("-")),
            )
            .map_err(|e: XlsxError| e.to_string())?;
        provenance
            .write_string(
                4,
                1,
                if manifest.disjointness.disjoint {
                    "Yes"
                } else if manifest.disjointness.overlapping_dataset_keys == 0 {
                    "Unproven"
                } else {
                    "No"
                },
            )
            .map_err(|e: XlsxError| e.to_string())?;
        for (row, value) in [
            manifest.disjointness.train_artifacts,
            manifest.disjointness.holdout_artifacts,
            manifest.disjointness.overlapping_dataset_keys,
            manifest.disjointness.missing_train_dataset_keys,
            manifest.disjointness.missing_holdout_dataset_keys,
        ]
        .into_iter()
        .enumerate()
        {
            provenance
                .write_number_with_format(5 + row as u32, 1, value as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        let warnings = if manifest.warnings.is_empty() {
            "None".to_string()
        } else {
            manifest.warnings.join(" | ")
        };
        provenance
            .write_string_with_format(10, 1, &excel_safe_text(&warnings), &wrap_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        add_native_table(
            provenance,
            2,
            0,
            10,
            1,
            "CohortIntegritySummaryTable",
            &summary_headers,
            TableStyle::Medium2,
        )?;

        let lineage_headers = ["Campaign lineage", "Value"];
        for (offset, header) in lineage_headers.iter().enumerate() {
            provenance
                .write_string_with_format(2, 3 + offset as u16, *header, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        let lineage_labels = [
            "Campaign summary path",
            "Campaign summary SHA-256",
            "Manifest hash",
            "Candidate manifest hash",
            "Admitted VCET candidates",
            "Selected candidate ID",
            "Cross-universe admission (schema v4)",
            "Admitted candidate set",
            "Expected train datasets",
            "Expected holdout datasets",
            "Matched train artifacts",
            "Matched holdout artifacts",
            "Excluded out-of-scope artifacts",
        ];
        for (index, label) in lineage_labels.iter().enumerate() {
            provenance
                .write_string(3 + index as u32, 3, *label)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        if let Some(lineage) = &manifest.campaign_lineage {
            provenance
                .write_string_with_format(
                    3,
                    4,
                    &excel_safe_text(&lineage.campaign_summary_path),
                    &wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            provenance
                .write_string(4, 4, &excel_safe_text(&lineage.campaign_summary_sha256))
                .map_err(|e: XlsxError| e.to_string())?;
            provenance
                .write_string(5, 4, &excel_safe_text(&lineage.manifest_hash))
                .map_err(|e: XlsxError| e.to_string())?;
            provenance
                .write_string(
                    6,
                    4,
                    &excel_safe_text(lineage.candidate_manifest_hash.as_deref().unwrap_or("-")),
                )
                .map_err(|e: XlsxError| e.to_string())?;
            provenance
                .write_number_with_format(7, 4, lineage.admitted_candidate_count as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            provenance
                .write_string(
                    8,
                    4,
                    &excel_safe_text(lineage.selected_candidate_id.as_deref().unwrap_or("-")),
                )
                .map_err(|e: XlsxError| e.to_string())?;
            let policy = lineage.cross_universe_admission;
            provenance
                .write_string_with_format(
                    9,
                    4,
                    &excel_safe_text(&format!(
                        "profitable >= {:.1}% | hard gate >= {:.1}% | PnL p10 >= {:.6} | Sharpe p10 >= {:.4} | p10 requested-tail: missing/failed rank below finite; unavailable fails admission",
                        policy.min_profitable_share_requested * 100.0,
                        policy.min_hard_gate_pass_share_requested * 100.0,
                        policy.min_pnl_p10,
                        policy.min_sharpe_p10,
                    )),
                    &wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
            let admitted_set = if lineage.admitted_candidates.is_empty() {
                "-".to_string()
            } else {
                lineage
                    .admitted_candidates
                    .iter()
                    .map(|candidate| {
                        format!(
                            "{}/{} [{}]",
                            candidate.fast_len, candidate.slow_len, candidate.candidate_id
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | ")
            };
            provenance
                .write_string_with_format(10, 4, &excel_safe_text(&admitted_set), &wrap_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            for (row, value) in [
                lineage.expected_train_datasets,
                lineage.expected_holdout_datasets,
                lineage.matched_train_artifacts,
                lineage.matched_holdout_artifacts,
                lineage.excluded_out_of_scope_artifacts,
            ]
            .into_iter()
            .enumerate()
            {
                provenance
                    .write_number_with_format(11 + row as u32, 4, value as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
        } else {
            for row in 3..=15 {
                provenance
                    .write_string(row, 4, if row == 3 { "Not bound" } else { "-" })
                    .map_err(|e: XlsxError| e.to_string())?;
            }
        }
        add_native_table(
            provenance,
            2,
            3,
            15,
            4,
            "CampaignLineageSummaryTable",
            &lineage_headers,
            TableStyle::Medium4,
        )?;

        let artifact_headers: [&str; 9] = [
            "Role",
            "Dataset",
            "Timeframe",
            "CSV path",
            "Tags",
            "Artifact path",
            "SHA-256",
            "Source fingerprint",
            "Timestamp",
        ];
        for (col, header) in artifact_headers.iter().enumerate() {
            provenance
                .write_string_with_format(17, col as u16, *header, &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        let mut artifact_row = 18u32;
        for artifact in manifest
            .train_artifacts
            .iter()
            .chain(manifest.holdout_artifacts.iter())
        {
            let tags = if artifact.tags.is_empty() {
                "-".to_string()
            } else {
                artifact.tags.join(", ")
            };
            let values = [
                excel_safe_text(&artifact.role),
                excel_safe_text(artifact.dataset_label.as_deref().unwrap_or("-")),
                excel_safe_text(artifact.timeframe.as_deref().unwrap_or("-")),
                excel_safe_text(artifact.csv_path.as_deref().unwrap_or("-")),
                excel_safe_text(&tags),
                excel_safe_text(&artifact.path),
                excel_safe_text(artifact.artifact_sha256.as_deref().unwrap_or("-")),
                excel_safe_text(
                    artifact
                        .source_content_fingerprint
                        .as_deref()
                        .unwrap_or("-"),
                ),
                excel_safe_text(artifact.ts.as_deref().unwrap_or("-")),
            ];
            for (col, value) in values.iter().enumerate() {
                provenance
                    .write_string_with_format(artifact_row, col as u16, value, &wrap_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
            }
            artifact_row += 1;
        }
        if artifact_row == 18 {
            provenance
                .merge_range(
                    18,
                    0,
                    18,
                    8,
                    "No artifact references were recorded for this cohort.",
                    &wrap_fmt,
                )
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            add_native_table(
                provenance,
                17,
                0,
                artifact_row - 1,
                8,
                "CohortArtifactsTable",
                &artifact_headers,
                TableStyle::Medium9,
            )?;
        }

        // Columns A/B and D/E also carry the summary tables above the
        // artifact listing. Give those label/value pairs enough room to stay
        // legible at normal zoom even though the artifact table uses them for
        // shorter role/dataset and path/tag values.
        let provenance_widths: [f64; 9] = [28.0, 32.0, 14.0, 40.0, 36.0, 42.0, 24.0, 30.0, 24.0];
        for (col, width) in provenance_widths.iter().enumerate() {
            let _ = provenance.set_column_width(col as u16, *width);
        }
        provenance
            .set_freeze_panes(18, 2)
            .map_err(|e: XlsxError| e.to_string())?;
        provenance
            .set_repeat_rows(17, 17)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if let Some(lineage) = cohort_manifest.and_then(|manifest| manifest.campaign_lineage.as_ref()) {
        add_vcet_candidate_evidence_sheet(
            &mut workbook,
            lineage,
            &title_fmt,
            &header_fmt,
            &label_fmt,
            &wrap_fmt,
            &int_fmt,
            &num6_fmt,
            &pct_fmt,
        )?;
    }

    {
        let dist = workbook.add_worksheet();
        dist.set_name("Distributions")
            .map_err(|e: XlsxError| e.to_string())?;
        apply_shareable_sheet_layout(dist, "VectorGrid Meta Analysis - Distributions");
        dist.merge_range(0, 0, 0, 13, "Meta metric distributions", &title_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.set_freeze_panes(3, 0)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.set_repeat_rows(2, 2)
            .map_err(|e: XlsxError| e.to_string())?;

        let final_scores: Vec<f64> = meta
            .groups
            .iter()
            .map(|g| g.final_score.unwrap_or(g.meta_score))
            .collect();
        let robust_p10s: Vec<f64> = meta.groups.iter().map(|g| g.robust_score_p10).collect();
        let pass_rates: Vec<f64> = meta.groups.iter().map(|g| g.pass_rate).collect();
        let dd_p90s: Vec<f64> = meta.groups.iter().map(|g| g.max_dd_p90).collect();

        let h1 = write_histogram_table(
            dist,
            2,
            0,
            "Final score",
            &final_scores,
            &header_fmt,
            &num6_fmt,
            &int_fmt,
        )?;
        let h2 = write_histogram_table(
            dist,
            2,
            3,
            "Robust p10",
            &robust_p10s,
            &header_fmt,
            &num6_fmt,
            &int_fmt,
        )?;
        let h3 = write_histogram_table(
            dist,
            2,
            6,
            "Pass rate",
            &pass_rates,
            &header_fmt,
            &pct_fmt,
            &int_fmt,
        )?;
        let h4 = write_histogram_table(
            dist,
            2,
            9,
            "MaxDD p90",
            &dd_p90s,
            &header_fmt,
            &pct_fmt,
            &int_fmt,
        )?;

        let _ = dist.set_column_width(0, 12.0);
        let _ = dist.set_column_width(1, 8.0);
        let _ = dist.set_column_width(3, 12.0);
        let _ = dist.set_column_width(4, 8.0);
        let _ = dist.set_column_width(6, 12.0);
        let _ = dist.set_column_width(7, 8.0);
        let _ = dist.set_column_width(9, 12.0);
        let _ = dist.set_column_width(10, 8.0);

        insert_hist_chart(
            dist,
            &h1,
            "Final score distribution",
            2,
            12,
            "#2563EB",
            "0.000",
        )?;
        insert_hist_chart(
            dist,
            &h2,
            "Robust p10 distribution",
            2,
            22,
            "#0F766E",
            "0.000",
        )?;
        insert_hist_chart(
            dist,
            &h3,
            "Pass rate distribution",
            20,
            12,
            "#16A34A",
            "0.0%",
        )?;
        insert_hist_chart(
            dist,
            &h4,
            "MaxDD p90 distribution",
            20,
            22,
            "#DC2626",
            "0.0%",
        )?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e: rust_xlsxwriter::XlsxError| e.to_string())
}

fn table_columns(headers: &[&str]) -> Vec<TableColumn> {
    headers
        .iter()
        .map(|header| TableColumn::new().set_header(*header))
        .collect()
}

#[allow(clippy::too_many_arguments)]
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
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn apply_shareable_sheet_layout(sheet: &mut Worksheet, title: &str) {
    sheet
        .set_screen_gridlines(false)
        .set_print_gridlines(false)
        .set_landscape()
        .set_print_fit_to_pages(1, 0)
        .set_print_center_horizontally(true)
        .set_zoom(90)
        .set_header(format!("&C{title}"))
        .set_footer("&LVectorGrid&CPage &P of &N&R&D");
}

fn matrix_dataset_collision_labels(
    groups: &[ta_app_core::MetaCandidateAggregate],
) -> BTreeSet<String> {
    let mut identities_by_display: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for group in groups {
        for row in &group.train_rows {
            let display = share_safe_dataset_name(&row.dataset).to_lowercase();
            identities_by_display
                .entry(display)
                .or_default()
                .insert(format!("{}\0{}", row.dataset, row.dataset_key));
        }
    }

    let reserved_headers = ["rank", "meta score", "final score"];
    let colliding_displays: BTreeSet<String> = identities_by_display
        .into_iter()
        .filter_map(|(display, identities)| {
            (identities.len() > 1 || reserved_headers.contains(&display.as_str()))
                .then_some(display)
        })
        .collect();

    groups
        .iter()
        .flat_map(|group| group.train_rows.iter())
        .filter_map(|row| {
            colliding_displays
                .contains(&share_safe_dataset_name(&row.dataset).to_lowercase())
                .then_some(row.dataset.clone())
        })
        .collect()
}

fn holdout_dataset_collision_labels(
    groups: &[ta_app_core::MetaCandidateAggregate],
) -> BTreeSet<String> {
    let mut identities_by_display: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for group in groups {
        for row in &group.holdout_rows {
            identities_by_display
                .entry(share_safe_dataset_name(&row.dataset).to_lowercase())
                .or_default()
                .insert(format!("{}\0{}", row.dataset, row.dataset_key));
        }
    }
    let colliding_displays: BTreeSet<String> = identities_by_display
        .into_iter()
        .filter_map(|(display, identities)| (identities.len() > 1).then_some(display))
        .collect();
    groups
        .iter()
        .flat_map(|group| group.holdout_rows.iter())
        .filter_map(|row| {
            colliding_displays
                .contains(&share_safe_dataset_name(&row.dataset).to_lowercase())
                .then_some(row.dataset.clone())
        })
        .collect()
}

fn holdout_dataset_label(
    row: &ta_app_core::HoldoutDatasetRow,
    collision_labels: &BTreeSet<String>,
) -> String {
    let display = share_safe_dataset_name(&row.dataset);
    if collision_labels.contains(&row.dataset)
        || display.chars().count() > ASSET_MATRIX_HEADER_CHAR_LIMIT
    {
        let suffix = if row.dataset_key.trim().is_empty() {
            format!(
                " [#{}]",
                stable_dataset_identity_suffix(&row.dataset, &row.dataset_key)
            )
        } else {
            format!(" [{}]", concise_text(&row.dataset_key, 32))
        };
        let display_budget = ASSET_MATRIX_HEADER_CHAR_LIMIT.saturating_sub(suffix.chars().count());
        format!("{}{suffix}", concise_text(&display, display_budget))
    } else {
        display
    }
}

fn matrix_dataset_label(row: &TrainDatasetRow, collision_labels: &BTreeSet<String>) -> String {
    let dataset = share_safe_dataset_name(&row.dataset);
    if collision_labels.contains(&row.dataset) {
        let key = concise_text(&share_safe_dataset_key(&row.dataset_key), 24);
        let suffix = format!(
            " [{key} · {}]",
            stable_dataset_identity_suffix(&row.dataset, &row.dataset_key)
        );
        let dataset_budget = ASSET_MATRIX_HEADER_CHAR_LIMIT.saturating_sub(suffix.chars().count());
        format!("{}{suffix}", concise_text(&dataset, dataset_budget))
    } else if dataset.chars().count() > ASSET_MATRIX_HEADER_CHAR_LIMIT {
        let suffix = format!(
            " [#{}]",
            stable_dataset_identity_suffix(&row.dataset, &row.dataset_key)
        );
        let dataset_budget = ASSET_MATRIX_HEADER_CHAR_LIMIT.saturating_sub(suffix.chars().count());
        format!("{}{suffix}", concise_text(&dataset, dataset_budget))
    } else {
        dataset
    }
}

fn validate_matrix_dataset_headers(datasets: &BTreeSet<String>) -> Result<(), String> {
    let mut seen = BTreeSet::from([
        "rank".to_string(),
        "meta score".to_string(),
        "final score".to_string(),
    ]);
    for dataset in datasets {
        let folded = dataset.to_lowercase();
        if !seen.insert(folded) {
            return Err(format!(
                "asset matrix dataset header remains ambiguous after share-safe normalization: {}",
                excel_safe_text(dataset)
            ));
        }
    }
    Ok(())
}

fn median_train_pnl_pct_by_dataset(
    rows: &[TrainDatasetRow],
    collision_labels: &BTreeSet<String>,
) -> BTreeMap<String, f64> {
    let mut map: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in rows.iter().filter(|row| row.pnl_pct.is_finite()) {
        map.entry(matrix_dataset_label(r, collision_labels))
            .or_default()
            .push(r.pnl_pct);
    }

    let mut out: BTreeMap<String, f64> = BTreeMap::new();
    for (k, mut vs) in map {
        vs.sort_by(|a, b| a.total_cmp(b));
        out.insert(k, median_sorted(&vs));
    }
    out
}

fn median_sorted(sorted: &[f64]) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    }
}

fn histogram(values: &[f64], bins: usize) -> Vec<(f64, usize)> {
    let values: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    if values.is_empty() {
        return Vec::new();
    }
    let bins = bins.clamp(1, 24);
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for &v in &values {
        if v < min_v {
            min_v = v;
        }
        if v > max_v {
            max_v = v;
        }
    }
    if !min_v.is_finite() || !max_v.is_finite() {
        return Vec::new();
    }
    if (max_v - min_v).abs() <= f64::EPSILON {
        return vec![(min_v, values.len())];
    }

    let width = (max_v - min_v) / bins as f64;
    let mut counts = vec![0usize; bins];
    for &v in &values {
        let mut idx = ((v - min_v) / width).floor() as usize;
        if idx >= bins {
            idx = bins - 1;
        }
        counts[idx] = counts[idx].saturating_add(1);
    }

    let mut out: Vec<(f64, usize)> = Vec::with_capacity(bins);
    for (i, count) in counts.iter().enumerate() {
        let center = min_v + (i as f64 + 0.5) * width;
        out.push((center, *count));
    }
    out
}

fn write_histogram_table(
    worksheet: &mut Worksheet,
    row0: u32,
    col0: u16,
    title: &str,
    values: &[f64],
    header_fmt: &rust_xlsxwriter::Format,
    num_fmt: &rust_xlsxwriter::Format,
    int_fmt: &rust_xlsxwriter::Format,
) -> Result<Option<(u32, u32, u16, u16, usize)>, String> {
    use rust_xlsxwriter::XlsxError;

    let hist = histogram(values, 10);
    worksheet
        .write_string_with_format(row0, col0, title, header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(row0, col0 + 1, "Count", header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    if hist.is_empty() {
        worksheet
            .write_string(row0 + 1, col0, "No data")
            .map_err(|e: XlsxError| e.to_string())?;
        return Ok(None);
    }

    for (i, (center, count)) in hist.iter().enumerate() {
        let r = row0 + 1 + i as u32;
        worksheet
            .write_number_with_format(r, col0, *center, num_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, col0 + 1, *count as f64, int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let max_count = hist.iter().map(|(_, count)| *count).max().unwrap_or(0);
    Ok(Some((
        row0 + 1,
        row0 + hist.len() as u32,
        col0,
        col0 + 1,
        max_count,
    )))
}

fn histogram_count_major_unit(max_count: usize) -> f64 {
    if max_count <= 5 {
        1.0
    } else {
        (max_count as f64 / 5.0).ceil()
    }
}

fn insert_hist_chart(
    worksheet: &mut Worksheet,
    hist_ref: &Option<(u32, u32, u16, u16, usize)>,
    title: &str,
    row: u32,
    col: u16,
    series_color: &str,
    x_num_format: &str,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    let Some((rmin, rmax, cbin, ccnt, max_count)) = hist_ref else {
        return Ok(());
    };

    let mut chart = Chart::new(ChartType::Column);
    chart
        .add_series()
        .set_categories(("Distributions", *rmin, *cbin, *rmax, *cbin))
        .set_values(("Distributions", *rmin, *ccnt, *rmax, *ccnt))
        .set_format(
            ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color(series_color)),
        );
    chart.title().set_name(title);
    chart
        .x_axis()
        .set_name("Bin center")
        .set_num_format(x_num_format)
        .set_major_gridlines(false);
    chart
        .y_axis()
        .set_name("Count")
        .set_num_format("0")
        .set_major_unit(histogram_count_major_unit(*max_count));
    chart.legend().set_hidden();
    chart.set_width(480).set_height(280);
    worksheet
        .insert_chart_with_offset(row, col, &chart, 10, 10)
        .map_err(|e: XlsxError| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::commands_meta::{
        MetaCampaignCohortLineage, MetaCohortArtifactRef, MetaCohortDisjointnessSummary,
        MetaHoldoutGroupExecutionEvidence, MetaVcetAdmittedCandidate, MetaVcetCandidateEvidenceRow,
        MetaVcetCandidateEvidenceSummary, MetaVcetEffectiveControls, MetaVcetSearchProvenance,
    };
    use serde_json::json;
    use std::io::{Cursor, Read};
    use ta_app_core::{
        HoldoutAggregate, HoldoutDatasetRow, HoldoutSummary, MetaCandidateAggregate,
        TrainDatasetRow,
    };
    use zip::ZipArchive;

    fn sample_params() -> serde_json::Value {
        json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "ema",
            "fast_len": 12,
            "slow_len": 48
        })
    }

    fn sample_group(
        key: &str,
        train_rows: Vec<TrainDatasetRow>,
        holdout_rows: Vec<HoldoutDatasetRow>,
        holdout: Option<HoldoutAggregate>,
    ) -> MetaCandidateAggregate {
        MetaCandidateAggregate {
            params: sample_params(),
            key: key.to_string(),
            ma_source: "close".to_string(),
            feature_context: None,
            run_context: None,
            fast_ma_params: None,
            slow_ma_params: None,
            representatives: vec![sample_params()],
            samples: 3,
            effective_samples_lower_bound: 3,
            datasets_covered: 3,
            pass_rate: 1.0,
            robust_score_median: 0.42,
            robust_score_p10: 0.21,
            evidence_flags: Vec::new(),
            tail_gap: 0.21,
            max_dd_p90: 0.18,
            cross_score_median: Some(0.17),
            psr_median: 0.63,
            dsr_median: 0.51,
            meta_score: 0.33,
            final_score: Some(0.36),
            holdout,
            holdout_coverage: None,
            train_rows,
            holdout_rows,
        }
    }

    fn sample_train_row(dataset: &str, dataset_key: &str, pnl_pct: f64) -> TrainDatasetRow {
        TrainDatasetRow {
            dataset: dataset.to_string(),
            dataset_key: dataset_key.to_string(),
            robust_score: 0.42,
            pnl: pnl_pct,
            pnl_pct,
            sharpe: 0.8,
            max_dd: 0.18,
            trades: 12.0,
            exposure: 0.55,
            cross_score: 0.17,
            cross_pnl_positive_frac: 0.66,
            psr: 0.63,
            dsr: 0.51,
            ts: Some("2026-02-17T00:00:00Z".to_string()),
        }
    }

    fn sample_holdout_row(dataset: &str) -> HoldoutDatasetRow {
        HoldoutDatasetRow {
            dataset: dataset.to_string(),
            dataset_key: format!(
                "source-fnv1a64-{}",
                stable_dataset_identity_suffix(dataset, "")
            ),
            score_median: 0.10,
            score_p10: -0.07,
            sharpe_median: 0.2,
            max_dd_worst: 0.35,
            pnl_median: 0.05,
            pass: true,
        }
    }

    fn sample_meta(
        groups: Vec<MetaCandidateAggregate>,
        holdout: Option<HoldoutSummary>,
    ) -> MetaSelectionResult {
        MetaSelectionResult {
            runtime_ms: 42,
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("1m".to_string()),
            artifacts_considered: 3,
            artifacts_used: 3,
            candidates_considered: 12,
            groups,
            warnings: vec!["smoke".to_string()],
            holdout,
            holdout_coverage: None,
        }
    }

    fn sample_holdout_execution(group_key: &str) -> MetaHoldoutExecutionEvidence {
        MetaHoldoutExecutionEvidence {
            requested_backend: "GpuKernel".to_string(),
            require_gpu: false,
            gpu_device_id: Some(0),
            datasets_requested: 4,
            datasets_evaluated: 3,
            datasets_failed: 1,
            gpu_datasets: 2,
            cpu_datasets: 0,
            hybrid_datasets: 1,
            fallback_datasets: 0,
            configured_auxiliary_stages: vec!["perturb".to_string(), "monte_carlo".to_string()],
            pass_rate_denominator: "requested_unique_holdout_datasets".to_string(),
            profitable_rate_denominator: "requested_unique_holdout_datasets".to_string(),
            route_validation_batch_counts: BTreeMap::from([
                ("GpuKernel".to_string(), 2),
                ("GpuKernel+CpuAuxiliary".to_string(), 1),
            ]),
            group_metrics: vec![MetaHoldoutGroupExecutionEvidence {
                group_key: group_key.to_string(),
                datasets_requested: 4,
                datasets_evaluated: 3,
                datasets_failed: 1,
                passes: 2,
                profitable: 3,
                evaluation_rate: 0.75,
                pass_rate: 0.5,
                profitable_rate: 0.75,
            }],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_vcet_candidate_aggregate(
        candidate_id: &str,
        fast_len: u16,
        slow_len: u16,
        requested: usize,
        evaluated: usize,
        profitable: usize,
        hard_gate_pass: usize,
        pnl_p10: Option<f64>,
        pnl_median: Option<f64>,
        sharpe_p10: Option<f64>,
        sharpe_median: Option<f64>,
        max_drawdown_p90: Option<f64>,
    ) -> ResearchCampaignFixedCandidateAggregate {
        ResearchCampaignFixedCandidateAggregate {
            candidate_id: candidate_id.to_string(),
            fast_len,
            slow_len,
            requested_datasets: requested,
            evaluated_datasets: evaluated,
            missing_or_failed_datasets: requested.saturating_sub(evaluated),
            profitable_datasets: profitable,
            hard_gate_pass_datasets: hard_gate_pass,
            profitable_share_requested: if requested == 0 {
                0.0
            } else {
                profitable as f64 / requested as f64
            },
            hard_gate_pass_share_requested: if requested == 0 {
                0.0
            } else {
                hard_gate_pass as f64 / requested as f64
            },
            pnl_p10,
            pnl_median,
            sharpe_p10,
            sharpe_median,
            max_drawdown_p90,
            trades_median: Some(24.0),
        }
    }

    fn sample_vcet_candidate_evidence() -> MetaVcetCandidateEvidenceSummary {
        let selected_id = "vcet_candidate_selected";
        let secondary_id = "vcet_candidate_secondary";
        let selected_training = sample_vcet_candidate_aggregate(
            selected_id,
            6,
            24,
            10,
            9,
            7,
            6,
            Some(0.015),
            Some(0.08),
            Some(0.35),
            Some(0.80),
            Some(0.12),
        );
        let selected_holdout = sample_vcet_candidate_aggregate(
            selected_id,
            6,
            24,
            4,
            4,
            3,
            2,
            Some(-0.005),
            Some(0.03),
            Some(0.10),
            Some(0.32),
            Some(0.18),
        );
        let selected_all = sample_vcet_candidate_aggregate(
            selected_id,
            6,
            24,
            14,
            13,
            10,
            8,
            Some(0.002),
            Some(0.06),
            Some(0.20),
            Some(0.61),
            Some(0.15),
        );
        let secondary_training = sample_vcet_candidate_aggregate(
            secondary_id,
            12,
            48,
            10,
            8,
            6,
            5,
            None,
            Some(0.04),
            None,
            Some(0.52),
            Some(0.20),
        );

        let mut config = ta_app_core::vcet_search::VcetSearchConfig::default();
        config.budget = 2;
        config.seed = ta_app_core::vcet_search::MAX_SAFE_VCET_SEARCH_SEED;
        MetaVcetCandidateEvidenceSummary {
            schema_version: 4,
            candidate_manifest_hash: "candidate-manifest-sha256".to_string(),
            selection_cohort: "training".to_string(),
            selection_status: "selected".to_string(),
            require_gpu_kernel: true,
            source_candidate_count: 2,
            included_candidate_count: 2,
            omitted_candidate_count: 0,
            row_cap: VCET_CANDIDATE_EVIDENCE_ROW_CAP,
            search: Some(MetaVcetSearchProvenance {
                schema_version: 1,
                sequence: "halton_cranley_patterson".to_string(),
                search_id: "vcet_search_sha256_test".to_string(),
                seed: ta_app_core::vcet_search::MAX_SAFE_VCET_SEARCH_SEED,
                requested_budget: 2,
                effective_budget: 2,
                valid_search_space_size: "765432109876543210".to_string(),
                float_identity_scale: 1_000_000,
                halton_attempts: 9,
                invalid_period_pairs_discarded: 2,
                duplicate_candidates_discarded: 1,
                lexicographic_fallback_candidates: 0,
                exhaustive_candidates: 0,
                config: Some(config),
            }),
            candidates: vec![
                MetaVcetCandidateEvidenceRow {
                    manifest_ordinal: 0,
                    candidate_id: selected_id.to_string(),
                    selected: true,
                    admitted: true,
                    controls: MetaVcetEffectiveControls {
                        evidence_state: "sealed_search_manifest".to_string(),
                        vidya_short_period: Some(6),
                        vidya_long_period: Some(24),
                        vidya_alpha: Some(0.2),
                        ce_period: Some(22),
                        ce_mult: Some(3.0),
                        ce_use_close: Some(true),
                        require_vidya_slope: Some(true),
                        max_trade_bars: Some(120),
                    },
                    training: selected_training,
                    holdout: Some(selected_holdout),
                    all_universe: Some(selected_all),
                },
                MetaVcetCandidateEvidenceRow {
                    manifest_ordinal: 1,
                    candidate_id: secondary_id.to_string(),
                    selected: false,
                    admitted: true,
                    controls: MetaVcetEffectiveControls {
                        evidence_state: "sealed_search_manifest".to_string(),
                        vidya_short_period: Some(12),
                        vidya_long_period: Some(48),
                        vidya_alpha: Some(0.35),
                        ce_period: Some(30),
                        ce_mult: Some(2.5),
                        ce_use_close: Some(false),
                        require_vidya_slope: Some(false),
                        max_trade_bars: Some(180),
                    },
                    training: secondary_training,
                    holdout: None,
                    all_universe: None,
                },
            ],
        }
    }

    fn sample_cohort_manifest() -> MetaCohortManifest {
        MetaCohortManifest {
            train_artifacts: vec![MetaCohortArtifactRef {
                role: "train".to_string(),
                path: r"C:\VectorGrid\runs\train-aapl.json".to_string(),
                artifact_sha256: Some("train-sha256".to_string()),
                ts: Some("2026-02-17T00:00:00Z".to_string()),
                dataset_label: Some("AAPL".to_string()),
                csv_path: Some(r"C:\market-data\aapl.csv".to_string()),
                source_content_fingerprint: Some("source-aapl".to_string()),
                timeframe: Some("1m".to_string()),
                tags: vec!["campaign-cohort:abc123".to_string()],
            }],
            holdout_artifacts: vec![MetaCohortArtifactRef {
                role: "holdout".to_string(),
                path: r"C:\VectorGrid\runs\holdout-msft.json".to_string(),
                artifact_sha256: Some("holdout-sha256".to_string()),
                ts: Some("2026-02-18T00:00:00Z".to_string()),
                dataset_label: Some("MSFT".to_string()),
                csv_path: Some(r"C:\market-data\msft.csv".to_string()),
                source_content_fingerprint: Some("source-msft".to_string()),
                timeframe: Some("1m".to_string()),
                tags: vec!["campaign-cohort:abc123".to_string(), "holdout".to_string()],
            }],
            disjointness: MetaCohortDisjointnessSummary {
                train_artifacts: 1,
                holdout_artifacts: 1,
                overlapping_dataset_keys: 0,
                missing_train_dataset_keys: 0,
                missing_holdout_dataset_keys: 1,
                disjoint: false,
            },
            holdout_tag: Some("holdout".to_string()),
            campaign_lineage: Some(MetaCampaignCohortLineage {
                campaign_summary_path: r"C:\VectorGrid\campaigns\vcet-summary.json".to_string(),
                campaign_summary_sha256: "campaign-summary-sha256".to_string(),
                manifest_hash: "campaign-manifest-sha256".to_string(),
                candidate_manifest_hash: Some("candidate-manifest-sha256".to_string()),
                admitted_candidate_count: 2,
                admitted_candidates: vec![
                    MetaVcetAdmittedCandidate {
                        candidate_id: "vcet_candidate_selected".to_string(),
                        fast_len: 6,
                        slow_len: 24,
                    },
                    MetaVcetAdmittedCandidate {
                        candidate_id: "vcet_candidate_secondary".to_string(),
                        fast_len: 12,
                        slow_len: 48,
                    },
                ],
                cross_universe_admission: Default::default(),
                selected_candidate_id: Some("vcet_candidate_selected".to_string()),
                expected_train_datasets: 2,
                expected_holdout_datasets: 3,
                matched_train_artifacts: 1,
                matched_holdout_artifacts: 1,
                excluded_out_of_scope_artifacts: 4,
                vcet_candidate_evidence: Some(sample_vcet_candidate_evidence()),
            }),
            warnings: vec!["one holdout dataset was missing".to_string()],
        }
    }

    fn open_xlsx(bytes: Vec<u8>) -> ZipArchive<Cursor<Vec<u8>>> {
        ZipArchive::new(Cursor::new(bytes)).expect("xlsx zip")
    }

    fn read_zip_text(archive: &mut ZipArchive<Cursor<Vec<u8>>>, path: &str) -> String {
        let mut text = String::new();
        archive
            .by_name(path)
            .unwrap_or_else(|_| panic!("missing zip entry: {path}"))
            .read_to_string(&mut text)
            .unwrap_or_else(|_| panic!("failed reading zip entry: {path}"));
        text
    }

    fn numeric_cell_value(sheet_xml: &str, cell: &str) -> f64 {
        let marker = format!("<c r=\"{cell}\"");
        let cell_start = sheet_xml
            .find(&marker)
            .unwrap_or_else(|| panic!("missing numeric cell {cell}"));
        let cell_xml = &sheet_xml[cell_start..];
        let value_start = cell_xml
            .find("<v>")
            .unwrap_or_else(|| panic!("missing numeric value for cell {cell}"))
            + 3;
        let value_end = cell_xml[value_start..]
            .find("</v>")
            .unwrap_or_else(|| panic!("unterminated numeric value for cell {cell}"))
            + value_start;
        cell_xml[value_start..value_end]
            .parse::<f64>()
            .unwrap_or_else(|_| panic!("invalid numeric value for cell {cell}"))
    }

    #[test]
    fn meta_xlsx_limit_helpers_respect_excel_boundaries() {
        assert!(validate_sheet_row_budget("Meta Analysis", 6, XLSX_MAX_ROW_INDEX - 6).is_ok());
        assert!(validate_sheet_row_budget("Meta Analysis", 6, XLSX_MAX_ROW_INDEX - 5).is_err());
        assert!(validate_sheet_row_budget("Overflow", usize::MAX, 1).is_err());
        assert_eq!(
            holdout_execution_group_header_row(14).expect("base holdout group header"),
            17
        );
        assert_eq!(
            holdout_execution_group_header_row(15).expect("shifted holdout group header"),
            18
        );
        assert!(holdout_execution_group_header_row(usize::MAX).is_err());
        assert!(validate_sheet_row_budget(
            "Cohort Provenance",
            COHORT_PROVENANCE_ARTIFACT_HEADER_ROW,
            XLSX_MAX_ROW_INDEX - COHORT_PROVENANCE_ARTIFACT_HEADER_ROW,
        )
        .is_ok());
        assert!(validate_sheet_row_budget(
            "Cohort Provenance",
            COHORT_PROVENANCE_ARTIFACT_HEADER_ROW,
            XLSX_MAX_ROW_INDEX - COHORT_PROVENANCE_ARTIFACT_HEADER_ROW + 1,
        )
        .is_err());

        assert_eq!(
            asset_matrix_last_column(XLSX_MAX_COLUMN_INDEX - 2).expect("maximum matrix width"),
            XLSX_MAX_COLUMN_INDEX as u16
        );
        assert!(asset_matrix_last_column(XLSX_MAX_COLUMN_INDEX - 1).is_err());
        assert!(asset_matrix_last_column(usize::MAX).is_err());
        assert!(validate_asset_matrix_cell_budget(999, 1_997).is_ok());
        assert!(validate_asset_matrix_cell_budget(999, 1_998).is_err());
        assert!(validate_asset_matrix_cell_budget(usize::MAX, 1).is_err());
        assert!(validate_asset_matrix_cell_budget(1, usize::MAX).is_err());

        let safe = excel_safe_text(&"🚀".repeat(XLSX_CELL_CHAR_LIMIT + 8));
        assert_eq!(safe.chars().count(), XLSX_CELL_CHAR_LIMIT);
        assert!(safe.ends_with(TRUNCATION_SUFFIX));
        assert_eq!(histogram_count_major_unit(0), 1.0);
        assert_eq!(histogram_count_major_unit(5), 1.0);
        assert_eq!(histogram_count_major_unit(6), 2.0);
        assert_eq!(histogram_count_major_unit(1_000), 200.0);
    }

    #[test]
    fn meta_report_xlsx_smoke() {
        let meta = sample_meta(
            vec![sample_group(
                "k1",
                vec![sample_train_row("AAPL", "csv:aapl.csv", 0.12)],
                vec![sample_holdout_row("MSFT")],
                Some(HoldoutAggregate {
                    datasets: 2,
                    pass_rate: 0.5,
                    score_median: 0.12,
                    score_p10: -0.05,
                    sharpe_median: 0.22,
                    max_dd_p90: 0.30,
                    pnl_median: 0.08,
                }),
            )],
            Some(HoldoutSummary {
                datasets: 2,
                skipped: 0,
            }),
        );

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        assert!(bytes.len() > 1024);
    }

    #[test]
    fn meta_report_xlsx_surfaces_holdout_execution_and_requested_universe_rates() {
        let group = sample_group(
            "k1",
            vec![sample_train_row("AAPL", "csv:aapl.csv", 0.12)],
            vec![sample_holdout_row("MSFT")],
            Some(HoldoutAggregate {
                datasets: 3,
                pass_rate: 0.5,
                score_median: 0.12,
                score_p10: -0.05,
                sharpe_median: 0.22,
                max_dd_p90: 0.30,
                pnl_median: 0.08,
            }),
        );
        let evidence = sample_holdout_execution(&group.key);
        let meta = sample_meta(
            vec![group],
            Some(HoldoutSummary {
                datasets: 3,
                skipped: 1,
            }),
        );

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, Some(&evidence), None)
                .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);

        let workbook_xml = read_zip_text(&mut archive, "xl/workbook.xml");
        assert!(workbook_xml.contains("Holdout Execution"));

        let summary_xml = read_zip_text(&mut archive, "xl/worksheets/sheet1.xml");
        assert_eq!(numeric_cell_value(&summary_xml, "F4"), 4.0);
        assert_eq!(numeric_cell_value(&summary_xml, "I4"), 3.0);
        assert_eq!(numeric_cell_value(&summary_xml, "L4"), 1.0);
        assert_eq!(numeric_cell_value(&summary_xml, "I5"), 2.0);
        assert_eq!(numeric_cell_value(&summary_xml, "L5"), 0.0);
        assert_eq!(numeric_cell_value(&summary_xml, "O5"), 1.0);
        assert_eq!(numeric_cell_value(&summary_xml, "R5"), 0.0);
        assert_eq!(numeric_cell_value(&summary_xml, "U5"), 0.0);

        let execution_xml = read_zip_text(&mut archive, "xl/worksheets/sheet5.xml");
        assert!((numeric_cell_value(&execution_xml, "H19") - 0.75).abs() < 1e-12);
        assert!((numeric_cell_value(&execution_xml, "I19") - 0.5).abs() < 1e-12);
        assert!((numeric_cell_value(&execution_xml, "J19") - 0.75).abs() < 1e-12);
        assert!(execution_xml.contains("<conditionalFormatting sqref=\"H19:J19\""));

        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        for expected in [
            "Holdout requested",
            "Fully GPU datasets",
            "Hybrid datasets",
            "Configured auxiliary stages",
            "perturb, monte_carlo",
            "Evaluation rate",
            "Profitable rate",
            "Validation batches",
            "GpuKernel+CpuAuxiliary",
        ] {
            assert!(shared_strings.contains(expected), "missing {expected}");
        }

        let table_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.starts_with("xl/tables/table") && path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let mut tables_xml = String::new();
        for path in table_paths {
            tables_xml.push_str(&read_zip_text(&mut archive, &path));
        }
        for name in [
            "HoldoutExecutionSummaryTable",
            "HoldoutRouteCountsTable",
            "HoldoutGroupCoverageTable",
        ] {
            assert!(tables_xml.contains(&format!("name=\"{name}\"")));
        }
    }

    #[test]
    fn meta_report_xlsx_moves_holdout_group_table_below_large_route_table() {
        let group = sample_group(
            "k1",
            vec![sample_train_row("AAPL", "csv:aapl.csv", 0.12)],
            vec![sample_holdout_row("MSFT")],
            None,
        );
        let mut evidence = sample_holdout_execution(&group.key);
        evidence.route_validation_batch_counts = (0..15)
            .map(|index| (format!("GpuKernelRoute{index:02}"), index + 1))
            .collect();
        let meta = sample_meta(vec![group], None);

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, Some(&evidence), None)
                .expect("meta report xlsx with 15 routes");
        let mut archive = open_xlsx(bytes);

        let execution_xml = read_zip_text(&mut archive, "xl/worksheets/sheet5.xml");
        assert!(execution_xml.contains("xSplit=\"2\""));
        assert!(execution_xml.contains("ySplit=\"19\""));
        assert!(execution_xml.contains("<conditionalFormatting sqref=\"H20:J20\""));

        let table_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.starts_with("xl/tables/table") && path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let tables: Vec<String> = table_paths
            .iter()
            .map(|path| read_zip_text(&mut archive, path))
            .collect();
        let route_table = tables
            .iter()
            .find(|xml| xml.contains("name=\"HoldoutRouteCountsTable\""))
            .expect("holdout route table");
        assert!(route_table.contains("ref=\"E3:F18\""));
        let group_table = tables
            .iter()
            .find(|xml| xml.contains("name=\"HoldoutGroupCoverageTable\""))
            .expect("holdout group table");
        assert!(group_table.contains("ref=\"A19:J20\""));
    }

    #[test]
    fn meta_report_xlsx_adds_filterable_cohort_provenance_sheet() {
        let meta = sample_meta(
            vec![sample_group(
                "k1",
                vec![sample_train_row("AAPL", "csv:aapl.csv", 0.12)],
                vec![sample_holdout_row("MSFT")],
                None,
            )],
            None,
        );
        let manifest = sample_cohort_manifest();

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, Some(&manifest))
                .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);

        let workbook_xml = read_zip_text(&mut archive, "xl/workbook.xml");
        assert!(workbook_xml.contains("Cohort Provenance"));

        let provenance_xml = read_zip_text(&mut archive, "xl/worksheets/sheet5.xml");
        assert!(provenance_xml.contains("showGridLines=\"0\""));
        assert!(provenance_xml.contains("xSplit=\"2\""));
        assert!(provenance_xml.contains("ySplit=\"18\""));
        assert!(provenance_xml.contains("<tableParts count=\"3\""));
        assert_eq!(numeric_cell_value(&provenance_xml, "B6"), 1.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "B7"), 1.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "B8"), 0.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "B10"), 1.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "E8"), 2.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "E12"), 2.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "E13"), 3.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "E14"), 1.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "E15"), 1.0);
        assert_eq!(numeric_cell_value(&provenance_xml, "E16"), 4.0);

        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        for expected in [
            "Cohort provenance, disjointness, and campaign lineage",
            "Campaign summary path",
            "Campaign summary SHA-256",
            "campaign-summary-sha256",
            "Candidate manifest hash",
            "Admitted VCET candidates",
            "Selected candidate ID",
            "Cross-universe admission (schema v4)",
            "p10 requested-tail: missing/failed rank below finite; unavailable fails admission",
            "Admitted candidate set",
            "vcet_candidate_selected",
            "vcet_candidate_secondary",
            "Excluded out-of-scope artifacts",
            "Unproven",
            "C:\\VectorGrid\\campaigns\\vcet-summary.json",
            "C:\\VectorGrid\\runs\\train-aapl.json",
            "C:\\market-data\\msft.csv",
            "campaign-cohort:abc123, holdout",
            "Source fingerprint",
            "one holdout dataset was missing",
        ] {
            assert!(shared_strings.contains(expected), "missing {expected}");
        }

        let table_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.starts_with("xl/tables/table") && path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let mut tables_xml = String::new();
        for path in table_paths {
            tables_xml.push_str(&read_zip_text(&mut archive, &path));
        }
        for name in [
            "CohortIntegritySummaryTable",
            "CampaignLineageSummaryTable",
            "CohortArtifactsTable",
        ] {
            assert!(tables_xml.contains(&format!("name=\"{name}\"")));
        }
        assert!(tables_xml.contains("<autoFilter"));
    }

    #[test]
    fn meta_report_xlsx_adds_bounded_vcet_candidate_evidence_sheet() {
        let meta = sample_meta(
            vec![sample_group(
                "k1",
                vec![sample_train_row("AAPL", "csv:aapl.csv", 0.12)],
                vec![sample_holdout_row("MSFT")],
                None,
            )],
            None,
        );
        let manifest = sample_cohort_manifest();

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, Some(&manifest))
                .expect("meta report xlsx");
        // Optional maintainer fixture emission for artifact-tool/Excel visual
        // inspection. Normal test runs remain in-memory and side-effect free.
        if let Some(path) = std::env::var_os("VECTORGRID_META_XLSX_FIXTURE_OUT") {
            let path = std::path::PathBuf::from(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create fixture directory");
            }
            std::fs::write(&path, &bytes).expect("write VCET Meta XLSX fixture");
        }
        let mut archive = open_xlsx(bytes);

        let workbook_xml = read_zip_text(&mut archive, "xl/workbook.xml");
        assert!(workbook_xml.contains("VCET Candidate Evidence"));

        let evidence_xml = read_zip_text(&mut archive, "xl/worksheets/sheet6.xml");
        assert!(evidence_xml.contains("showGridLines=\"0\""));
        assert!(evidence_xml.contains("xSplit=\"2\""));
        assert!(evidence_xml.contains("ySplit=\"20\""));
        assert!(evidence_xml.contains("<tableParts count=\"2\""));
        assert!(evidence_xml.contains("<conditionalFormatting sqref=\"S21:T22\""));
        assert!(evidence_xml.contains("<conditionalFormatting sqref=\"AF21:AG22\""));
        assert!(evidence_xml.contains("<conditionalFormatting sqref=\"AQ21:AR22\""));
        assert!(evidence_xml.contains("<conditionalFormatting sqref=\"U21:U22\""));
        assert!(evidence_xml.contains("<conditionalFormatting sqref=\"W21:W22\""));
        assert!(evidence_xml.contains("<conditionalFormatting sqref=\"Y21:Y22\""));

        assert_eq!(numeric_cell_value(&evidence_xml, "F21"), 6.0);
        assert_eq!(numeric_cell_value(&evidence_xml, "G21"), 24.0);
        assert!((numeric_cell_value(&evidence_xml, "H21") - 0.2).abs() < 1e-12);
        assert_eq!(numeric_cell_value(&evidence_xml, "I21"), 22.0);
        assert_eq!(numeric_cell_value(&evidence_xml, "J21"), 3.0);
        assert_eq!(numeric_cell_value(&evidence_xml, "M21"), 120.0);
        assert!((numeric_cell_value(&evidence_xml, "S21") - 0.7).abs() < 1e-12);
        assert!((numeric_cell_value(&evidence_xml, "T21") - 0.6).abs() < 1e-12);
        assert!((numeric_cell_value(&evidence_xml, "U21") - 0.015).abs() < 1e-12);
        assert!((numeric_cell_value(&evidence_xml, "W21") - 0.35).abs() < 1e-12);
        assert!((numeric_cell_value(&evidence_xml, "AF21") - 0.75).abs() < 1e-12);
        assert_eq!(numeric_cell_value(&evidence_xml, "K4"), 4.0);
        assert_eq!(numeric_cell_value(&evidence_xml, "L4"), 48.0);
        assert_eq!(numeric_cell_value(&evidence_xml, "M4"), 2.0);
        assert!((numeric_cell_value(&evidence_xml, "K6") - 0.05).abs() < 1e-12);
        assert!((numeric_cell_value(&evidence_xml, "L6") - 0.8).abs() < 1e-12);
        assert!((numeric_cell_value(&evidence_xml, "M6") - 0.05).abs() < 1e-12);

        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        for expected in [
            "VCET candidate controls and cross-universe evidence",
            "Effective VCET controls",
            "Training cohort (selection evidence)",
            "Holdout cohort (not used for selection)",
            "sealed_search_manifest",
            "vcet_search_sha256_test",
            "halton_cranley_patterson",
            "9007199254740991",
            "765432109876543210",
            "vcet_candidate_selected",
            "vcet_candidate_secondary",
            "VIDYA alpha",
            "CE multiplier",
            "Search axis",
            "Step / quantum",
            "Require VIDYA slope",
            "Train profitable share",
            "Train hard-gate share",
            "Train PnL p10 (native)",
            "Train Sharpe p10",
            "Lexicographic fallback (legacy)",
            "Exhaustive candidates",
        ] {
            assert!(shared_strings.contains(expected), "missing {expected}");
        }

        let table_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.starts_with("xl/tables/table") && path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let mut tables_xml = String::new();
        for path in table_paths {
            tables_xml.push_str(&read_zip_text(&mut archive, &path));
        }
        assert!(tables_xml.contains("name=\"VcetCandidateEvidenceTable\""));
        assert!(tables_xml.contains("name=\"VcetSearchAxesTable\""));

        let chart_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.starts_with("xl/charts/chart") && path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let mut charts_xml = String::new();
        for path in chart_paths {
            charts_xml.push_str(&read_zip_text(&mut archive, &path));
        }
        assert!(charts_xml.contains("Training profitability share vs Sharpe p10"));
        assert!(charts_xml.contains("'VCET Candidate Evidence'!$S$21:$S$22"));
        assert!(charts_xml.contains("'VCET Candidate Evidence'!$W$21:$W$22"));
    }

    #[test]
    fn meta_report_xlsx_rejects_inconsistent_vcet_candidate_evidence_bounds() {
        let meta = sample_meta(Vec::new(), None);
        let mut manifest = sample_cohort_manifest();
        let evidence = manifest
            .campaign_lineage
            .as_mut()
            .and_then(|lineage| lineage.vcet_candidate_evidence.as_mut())
            .expect("VCET candidate evidence");
        evidence.row_cap = 1;

        let error =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, Some(&manifest))
                .expect_err("candidate evidence beyond its declared cap must fail closed");
        assert!(
            error.contains("declared cap 1"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn meta_report_xlsx_distinguishes_exhaustive_search_from_qmc_and_legacy_fallback() {
        let meta = sample_meta(Vec::new(), None);
        let mut manifest = sample_cohort_manifest();
        let search = manifest
            .campaign_lineage
            .as_mut()
            .and_then(|lineage| lineage.vcet_candidate_evidence.as_mut())
            .and_then(|evidence| evidence.search.as_mut())
            .expect("search provenance");
        search.sequence = "exhaustive_lexicographic".to_string();
        search.halton_attempts = 0;
        search.exhaustive_candidates = 2;
        search.lexicographic_fallback_candidates = 0;

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, Some(&manifest))
                .expect("exhaustive evidence workbook");
        let mut archive = open_xlsx(bytes);
        let evidence_xml = read_zip_text(&mut archive, "xl/worksheets/sheet6.xml");
        assert_eq!(numeric_cell_value(&evidence_xml, "E17"), 2.0);
        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        assert!(shared_strings.contains("Exhaustive lexicographic (sealed manifest)"));
        assert!(shared_strings.contains("Lexicographic fallback (legacy)"));
    }

    #[test]
    fn meta_report_xlsx_marks_missing_vcet_candidate_evidence_without_inference() {
        let meta = sample_meta(Vec::new(), None);
        let mut manifest = sample_cohort_manifest();
        manifest
            .campaign_lineage
            .as_mut()
            .expect("campaign lineage")
            .vcet_candidate_evidence = None;

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, Some(&manifest))
                .expect("legacy evidence workbook");
        let mut archive = open_xlsx(bytes);
        let evidence_xml = read_zip_text(&mut archive, "xl/worksheets/sheet6.xml");
        assert!(!evidence_xml.contains("<tableParts"));
        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        assert!(shared_strings.contains("Unavailable; no values inferred"));
        assert!(shared_strings.contains(
            "No sealed VCET candidate evidence is available; all candidate fields are intentionally blank rather than inferred."
        ));
    }

    #[test]
    fn meta_report_xlsx_contains_expected_sheet_names_and_chart_parts() {
        let meta = sample_meta(
            vec![sample_group(
                "k1",
                vec![sample_train_row("AAPL", "csv:aapl.csv", 0.12)],
                vec![sample_holdout_row("MSFT")],
                Some(HoldoutAggregate {
                    datasets: 2,
                    pass_rate: 0.5,
                    score_median: 0.12,
                    score_p10: -0.05,
                    sharpe_median: 0.22,
                    max_dd_p90: 0.30,
                    pnl_median: 0.08,
                }),
            )],
            Some(HoldoutSummary {
                datasets: 2,
                skipped: 0,
            }),
        );

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);

        let workbook_xml = read_zip_text(&mut archive, "xl/workbook.xml");
        assert!(workbook_xml.contains("Meta Analysis"));
        assert!(workbook_xml.contains("By Asset (Train)"));
        assert!(workbook_xml.contains("Asset Matrix"));
        assert!(workbook_xml.contains("Holdout Rows"));
        assert!(!workbook_xml.contains("Holdout Execution"));
        assert!(!workbook_xml.contains("Cohort Provenance"));
        assert!(workbook_xml.contains("Distributions"));

        let summary_xml = read_zip_text(&mut archive, "xl/worksheets/sheet1.xml");
        assert!(summary_xml.contains("<dimension"));

        let chart1 = read_zip_text(&mut archive, "xl/charts/chart1.xml");
        let chart2 = read_zip_text(&mut archive, "xl/charts/chart2.xml");
        let chart3 = read_zip_text(&mut archive, "xl/charts/chart3.xml");
        let chart4 = read_zip_text(&mut archive, "xl/charts/chart4.xml");
        let chart5 = read_zip_text(&mut archive, "xl/charts/chart5.xml");
        let chart6 = read_zip_text(&mut archive, "xl/charts/chart6.xml");

        assert!(chart1.contains("Final score by rank"));
        assert!(chart2.contains("Pass rate vs robust p10"));
        assert!(chart3.contains("Final score distribution"));
        assert!(chart4.contains("Robust p10 distribution"));
        assert!(chart5.contains("Pass rate distribution"));
        assert!(chart6.contains("MaxDD p90 distribution"));
    }

    #[test]
    fn meta_report_xlsx_adds_tables_heatmap_and_semantic_formats() {
        let meta = sample_meta(
            vec![
                sample_group(
                    "k1",
                    vec![
                        sample_train_row("AAPL", "csv:aapl.csv", 0.12),
                        sample_train_row("MSFT", "csv:msft.csv", -0.06),
                    ],
                    vec![sample_holdout_row("NVDA")],
                    Some(HoldoutAggregate {
                        datasets: 1,
                        pass_rate: 1.0,
                        score_median: 0.12,
                        score_p10: 0.03,
                        sharpe_median: 0.22,
                        max_dd_p90: 0.30,
                        pnl_median: 0.08,
                    }),
                ),
                sample_group(
                    "k2",
                    vec![
                        sample_train_row("AAPL", "csv:aapl.csv", 0.03),
                        sample_train_row("MSFT", "csv:msft.csv", 0.09),
                    ],
                    vec![sample_holdout_row("NVDA")],
                    None,
                ),
            ],
            Some(HoldoutSummary {
                datasets: 1,
                skipped: 0,
            }),
        );

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);

        let table_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.starts_with("xl/tables/table") && path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        assert_eq!(table_paths.len(), 4);
        let mut tables_xml = String::new();
        for path in table_paths {
            tables_xml.push_str(&read_zip_text(&mut archive, &path));
        }
        for name in [
            "MetaAnalysisTable",
            "ByAssetTrainTable",
            "AssetMatrixTable",
            "HoldoutRowsTable",
        ] {
            assert!(tables_xml.contains(&format!("name=\"{name}\"")));
        }

        let styles_xml = read_zip_text(&mut archive, "xl/styles.xml");
        assert!(styles_xml.contains("0.00%;[Red](0.00%);-"));

        let summary_xml = read_zip_text(&mut archive, "xl/worksheets/sheet1.xml");
        assert!(summary_xml.contains("showGridLines=\"0\""));
        assert!(summary_xml.contains("orientation=\"landscape\""));
        assert!(summary_xml.contains("<tablePart"));

        let train_xml = read_zip_text(&mut archive, "xl/worksheets/sheet2.xml");
        assert!(train_xml.contains("<tablePart"));

        let matrix_xml = read_zip_text(&mut archive, "xl/worksheets/sheet3.xml");
        assert!(matrix_xml.contains("<conditionalFormatting sqref=\"D4:E5\""));
        assert!(matrix_xml.contains("<colorScale>"));
        assert!(matrix_xml.contains("rgb=\"FFF8696B\""));
        assert!(matrix_xml.contains("rgb=\"FFFFEB84\""));
        assert!(matrix_xml.contains("rgb=\"FF63BE7B\""));
        assert!(matrix_xml.contains("<tablePart"));

        let holdout_xml = read_zip_text(&mut archive, "xl/worksheets/sheet4.xml");
        assert!(holdout_xml.contains("<tablePart"));

        let final_score_chart = read_zip_text(&mut archive, "xl/charts/chart1.xml");
        assert!(final_score_chart.contains("formatCode=\"0.000\""));
        assert!(final_score_chart.contains("val=\"2563EB\""));

        let pass_rate_chart = read_zip_text(&mut archive, "xl/charts/chart5.xml");
        assert!(pass_rate_chart.contains("formatCode=\"0.0%\""));
        assert!(pass_rate_chart.contains("val=\"16A34A\""));
    }

    #[test]
    fn median_train_pnl_pct_by_dataset_uses_true_even_median_and_collision_labels() {
        let collision_labels = BTreeSet::from(["AAPL".to_string()]);
        let rows = vec![
            sample_train_row("AAPL", "csv:aapl_1.csv", 0.10),
            sample_train_row("AAPL", "csv:aapl_1.csv", 0.30),
            sample_train_row("AAPL", "csv:aapl_2.csv", 0.20),
            sample_train_row("AAPL", "csv:aapl_2.csv", 0.40),
        ];

        let by_dataset = median_train_pnl_pct_by_dataset(&rows, &collision_labels);

        assert_eq!(by_dataset.len(), 2);
        let first_label = matrix_dataset_label(&rows[0], &collision_labels);
        let second_label = matrix_dataset_label(&rows[2], &collision_labels);
        let first = by_dataset
            .get(&first_label)
            .expect("first collision bucket");
        let second = by_dataset
            .get(&second_label)
            .expect("second collision bucket");
        assert!(first_label.contains("AAPL [csv:aapl_1.csv · "));
        assert!(second_label.contains("AAPL [csv:aapl_2.csv · "));
        assert!((*first - 0.20).abs() < 1e-12);
        assert!((*second - 0.30).abs() < 1e-12);
    }

    #[test]
    fn asset_matrix_labels_remain_unique_after_truncation_and_unicode_case_folding() {
        let prefix = "long_dataset_".repeat(10);
        let first = sample_train_row(
            &format!("{prefix}alpha"),
            &format!("csv:{}", "first_key_".repeat(20)),
            0.10,
        );
        let second = sample_train_row(
            &format!("{prefix}beta"),
            &format!("csv:{}", "second_key_".repeat(20)),
            0.20,
        );
        let first_label = matrix_dataset_label(&first, &BTreeSet::new());
        let second_label = matrix_dataset_label(&second, &BTreeSet::new());
        assert_ne!(first_label, second_label);
        assert!(first_label.chars().count() <= ASSET_MATRIX_HEADER_CHAR_LIMIT);
        assert!(second_label.chars().count() <= ASSET_MATRIX_HEADER_CHAR_LIMIT);
        assert!(first_label.contains("[#"));
        assert!(second_label.contains("[#"));

        let unicode_group = sample_group(
            "unicode",
            vec![
                sample_train_row("Ä", "csv:upper.csv", 0.10),
                sample_train_row("ä", "csv:lower.csv", 0.20),
            ],
            Vec::new(),
            None,
        );
        let collisions = matrix_dataset_collision_labels(std::slice::from_ref(&unicode_group));
        assert!(collisions.contains("Ä"));
        assert!(collisions.contains("ä"));
        let unicode_labels: BTreeSet<String> = unicode_group
            .train_rows
            .iter()
            .map(|row| matrix_dataset_label(row, &collisions))
            .collect();
        assert_eq!(unicode_labels.len(), 2);
        validate_matrix_dataset_headers(&unicode_labels).expect("Unicode-safe headers");
    }

    #[test]
    fn asset_matrix_medians_ignore_non_finite_pnl_values() {
        let rows = vec![
            sample_train_row("AAPL", "csv:aapl.csv", 0.10),
            sample_train_row("AAPL", "csv:aapl.csv", f64::NAN),
            sample_train_row("AAPL", "csv:aapl.csv", f64::INFINITY),
            sample_train_row("MSFT", "csv:msft.csv", f64::NEG_INFINITY),
        ];

        let by_dataset = median_train_pnl_pct_by_dataset(&rows, &BTreeSet::new());

        assert_eq!(by_dataset.get("AAPL"), Some(&0.10));
        assert!(!by_dataset.contains_key("MSFT"));
        assert!(by_dataset.values().all(|value| value.is_finite()));
    }

    #[test]
    fn meta_xlsx_renders_non_finite_metrics_as_missing_values() {
        let mut train = sample_train_row("AAPL", "csv:aapl.csv", f64::NAN);
        train.robust_score = f64::INFINITY;
        train.pnl = f64::NEG_INFINITY;
        train.sharpe = f64::NAN;
        train.max_dd = f64::INFINITY;
        train.trades = f64::NAN;
        train.exposure = f64::NEG_INFINITY;
        train.cross_score = f64::NAN;
        train.cross_pnl_positive_frac = f64::INFINITY;
        train.psr = f64::NEG_INFINITY;
        train.dsr = f64::NAN;

        let mut holdout_row = sample_holdout_row("MSFT");
        holdout_row.score_median = f64::NAN;
        holdout_row.score_p10 = f64::INFINITY;
        holdout_row.sharpe_median = f64::NEG_INFINITY;
        holdout_row.max_dd_worst = f64::NAN;
        holdout_row.pnl_median = f64::INFINITY;

        let mut group = sample_group("k1", vec![train], vec![holdout_row], None);
        group.pass_rate = f64::NAN;
        group.meta_score = f64::INFINITY;
        group.final_score = Some(f64::NEG_INFINITY);
        group.robust_score_p10 = f64::NAN;
        group.robust_score_median = f64::INFINITY;
        group.tail_gap = f64::NEG_INFINITY;
        group.max_dd_p90 = f64::NAN;
        group.cross_score_median = Some(f64::INFINITY);
        group.psr_median = f64::NEG_INFINITY;
        group.dsr_median = f64::NAN;

        let bytes = build_meta_report_xlsx_bytes(
            "2026-02-17T00:00:00Z",
            &sample_meta(vec![group], None),
            None,
            None,
        )
        .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);
        let xml_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let mut xml = String::new();
        for path in xml_paths {
            xml.push_str(&read_zip_text(&mut archive, &path));
        }

        assert!(!xml.contains(">NAN<"));
        assert!(!xml.contains(">INF<"));
        assert!(!xml.contains(">-INF<"));
    }

    #[test]
    fn meta_summary_charts_cap_large_group_sets_to_top_100() {
        let groups = (0..=SUMMARY_CHART_ROW_CAP)
            .map(|index| sample_group(&format!("k{index}"), Vec::new(), Vec::new(), None))
            .collect();
        let meta = sample_meta(groups, None);

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);
        let chart1 = read_zip_text(&mut archive, "xl/charts/chart1.xml");
        let chart2 = read_zip_text(&mut archive, "xl/charts/chart2.xml");

        assert!(chart1.contains("Final score by rank (top 100)"));
        assert!(chart2.contains("Pass rate vs robust p10 (top 100)"));
        assert!(chart1.contains("$A$8:$A$107"));
        assert!(!chart1.contains("$A$8:$A$108"));
        assert!(chart2.contains("$D$8:$D$107"));
        assert!(!chart2.contains("$D$8:$D$108"));
    }

    #[test]
    fn meta_xlsx_strips_private_dataset_paths_and_disambiguates_excel_headers() {
        let meta = sample_meta(
            vec![sample_group(
                "k1",
                vec![
                    sample_train_row("AAPL", r"csv:C:\private\one\data.csv", 0.10),
                    sample_train_row("aapl", r"csv:D:\secret\two\data.csv", 0.20),
                    sample_train_row("Rank", r"csv:C:\private\three\rank.csv", 0.30),
                ],
                Vec::new(),
                None,
            )],
            None,
        );

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);
        let xml_paths: Vec<String> = archive
            .file_names()
            .filter(|path| path.ends_with(".xml"))
            .map(str::to_owned)
            .collect();
        let mut xml = String::new();
        for path in xml_paths {
            xml.push_str(&read_zip_text(&mut archive, &path));
        }

        assert!(!xml.contains(r"C:\private"));
        assert!(!xml.contains(r"D:\secret"));
        assert!(xml.contains("Matrix dataset"));
        assert!(xml.contains("AAPL [csv:data.csv"));
        assert!(xml.contains("aapl [csv:data.csv"));
        assert!(xml.contains("Rank [csv:rank.csv"));
    }

    #[test]
    fn meta_xlsx_disambiguates_holdout_rows_with_the_same_share_safe_label() {
        let mut first = sample_holdout_row("AAPL.csv");
        first.dataset_key = "source-fnv1a64-1111111111111111".to_string();
        let mut second = sample_holdout_row("AAPL.csv");
        second.dataset_key = "source-fnv1a64-2222222222222222".to_string();
        let meta = sample_meta(
            vec![sample_group("k1", Vec::new(), vec![first, second], None)],
            None,
        );

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);
        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");

        assert_eq!(
            shared_strings.matches("AAPL.csv [source-fnv1a64-").count(),
            2
        );
        assert!(shared_strings.contains("source-fnv1a64-1111111111111111"));
        assert!(shared_strings.contains("source-fnv1a64-2222222222222222"));
        assert!(shared_strings.contains("Dataset identity"));
        assert!(shared_strings.contains("Dataset key"));
    }

    #[test]
    fn meta_report_xlsx_handles_no_train_or_holdout_rows_and_collision_headers() {
        let meta = sample_meta(
            vec![
                sample_group(
                    "k1",
                    vec![
                        sample_train_row("AAPL", "csv:aapl_primary.csv", 0.10),
                        sample_train_row("AAPL", "csv:aapl_primary.csv", 0.30),
                        sample_train_row("AAPL", "csv:aapl_alt.csv", 0.20),
                        sample_train_row("AAPL", "csv:aapl_alt.csv", 0.40),
                    ],
                    Vec::new(),
                    None,
                ),
                sample_group("k2", Vec::new(), Vec::new(), None),
            ],
            None,
        );

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);

        let workbook_xml = read_zip_text(&mut archive, "xl/workbook.xml");
        assert!(workbook_xml.contains("Holdout Rows"));

        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        assert!(shared_strings.contains("AAPL [csv:aapl_primary.csv · "));
        assert!(shared_strings.contains("AAPL [csv:aapl_alt.csv · "));
        assert!(shared_strings.contains("No holdout rows were saved for this run."));

        let asset_matrix_xml = read_zip_text(&mut archive, "xl/worksheets/sheet3.xml");
        assert!(asset_matrix_xml.contains("0.2"));
        assert!(asset_matrix_xml.contains("0.3"));
    }

    #[test]
    fn meta_report_xlsx_handles_fully_empty_train_and_holdout_sections() {
        let meta = sample_meta(vec![sample_group("k1", Vec::new(), Vec::new(), None)], None);

        let bytes = build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta, None, None)
            .expect("meta report xlsx");
        let mut archive = open_xlsx(bytes);

        let shared_strings = read_zip_text(&mut archive, "xl/sharedStrings.xml");
        assert!(shared_strings.contains("No training rows were captured for this meta run."));
        assert!(shared_strings.contains("No training rows available."));
        assert!(shared_strings.contains("No holdout rows were saved for this run."));
    }
}
