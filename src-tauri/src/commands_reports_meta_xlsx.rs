use rust_xlsxwriter::{Chart, ChartType, Worksheet};
use std::collections::{BTreeMap, BTreeSet};
use ta_app_core::{MetaSelectionResult, TrainDatasetRow};

pub(crate) fn build_meta_report_xlsx_bytes(
    ts: &str,
    meta: &MetaSelectionResult,
) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};

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
    let int_fmt = Format::new().set_num_format("0");
    let num3_fmt = Format::new().set_num_format("0.000");
    let num6_fmt = Format::new().set_num_format("0.000000");
    let pct_fmt = Format::new().set_num_format("0.0%");

    {
        let summary = workbook.add_worksheet();
        summary
            .set_name("Meta Analysis")
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .merge_range(0, 0, 0, 21, "VectorTA Stage 3 Meta Analysis", &title_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(1, 0, "ts", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_string(1, 1, ts)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(1, 3, "strategy_kind", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_string(1, 4, &meta.strategy_kind)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(1, 6, "timeframe", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_string(1, 7, meta.timeframe.as_deref().unwrap_or(""))
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(2, 0, "artifacts_considered", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(2, 1, meta.artifacts_considered as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(2, 3, "artifacts_used", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(2, 4, meta.artifacts_used as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(2, 6, "candidates_considered", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(2, 7, meta.candidates_considered as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        summary
            .write_string_with_format(3, 0, "runtime_ms", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        summary
            .write_number_with_format(3, 1, meta.runtime_ms as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        if let Some(h) = meta.holdout.as_ref() {
            summary
                .write_string_with_format(3, 3, "holdout_datasets", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(3, 4, h.datasets as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string_with_format(3, 6, "holdout_skipped", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(3, 7, h.skipped as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if !meta.warnings.is_empty() {
            let warnings_text = meta.warnings.join(" | ");
            summary
                .write_string_with_format(4, 0, "warnings", &label_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string(4, 1, &warnings_text)
                .map_err(|e: XlsxError| e.to_string())?;
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

        let widths: [f64; 22] = [
            6.0, 8.0, 9.0, 9.0, 11.0, 11.0, 10.0, 10.0, 9.0, 10.0, 10.0, 9.0, 9.0, 9.0, 10.0, 10.0,
            10.0, 11.0, 11.0, 9.0, 10.0, 80.0,
        ];
        for (c, w) in widths.iter().enumerate() {
            let _ = summary.set_column_width(c as u16, *w);
        }

        for (i, row) in meta.groups.iter().enumerate() {
            let r = header_row + 1 + i as u32;
            let final_score = row.final_score.unwrap_or(row.meta_score);
            let params_json = serde_json::to_string(&row.params).unwrap_or_default();

            summary
                .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 1, row.samples as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 2, row.datasets_covered as f64, &int_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 3, row.pass_rate, &pct_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 4, row.meta_score, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 5, final_score, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 6, row.robust_score_p10, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 7, row.robust_score_median, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 8, row.tail_gap, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 9, row.max_dd_p90, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 10, row.cross_score_median, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 11, row.psr_median, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_number_with_format(r, 12, row.dsr_median, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;

            if let Some(h) = row.holdout.as_ref() {
                summary
                    .write_number_with_format(r, 13, h.pass_rate, &pct_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_number_with_format(r, 14, h.score_p10, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_number_with_format(r, 15, h.score_median, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_number_with_format(r, 16, h.max_dd_p90, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_number_with_format(r, 17, h.sharpe_median, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                summary
                    .write_number_with_format(r, 18, h.pnl_median, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
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
                .write_string(r, 20, &row.ma_source)
                .map_err(|e: XlsxError| e.to_string())?;
            summary
                .write_string(r, 21, &params_json)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if !meta.groups.is_empty() {
            let row_min = header_row + 1;
            let row_max = header_row + meta.groups.len() as u32;

            let mut final_chart = Chart::new(ChartType::Column);
            final_chart
                .add_series()
                .set_categories(("Meta Analysis", row_min, 0, row_max, 0))
                .set_values(("Meta Analysis", row_min, 5, row_max, 5));
            final_chart.title().set_name("Final score by rank");
            final_chart.x_axis().set_name("Rank");
            final_chart.y_axis().set_name("Final score");
            final_chart.legend().set_hidden();
            summary
                .insert_chart_with_offset(1, 24, &final_chart, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;

            let mut scatter = Chart::new(ChartType::Scatter);
            scatter
                .add_series()
                .set_name("Pass vs robust")
                .set_categories(("Meta Analysis", row_min, 3, row_max, 3))
                .set_values(("Meta Analysis", row_min, 6, row_max, 6));
            scatter.title().set_name("Pass rate vs robust p10");
            scatter.x_axis().set_name("Pass rate");
            scatter.y_axis().set_name("Robust p10");
            scatter.legend().set_hidden();
            summary
                .insert_chart_with_offset(20, 24, &scatter, 10, 10)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    {
        let by_asset = workbook.add_worksheet();
        by_asset
            .set_name("By Asset (Train)")
            .map_err(|e: XlsxError| e.to_string())?;
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
            "Dataset",
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
        let widths: [f64; 16] = [
            10.0, 18.0, 28.0, 11.0, 10.0, 10.0, 9.0, 9.0, 8.0, 8.0, 10.0, 12.0, 8.0, 8.0, 24.0,
            70.0,
        ];
        for (c, w) in widths.iter().enumerate() {
            let _ = by_asset.set_column_width(c as u16, *w);
        }

        let mut row_ix = header_row + 1;
        for (i, g) in meta.groups.iter().enumerate() {
            let params_json = serde_json::to_string(&g.params).unwrap_or_default();
            for tr in &g.train_rows {
                by_asset
                    .write_number_with_format(row_ix, 0, (i + 1) as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_string(row_ix, 1, &tr.dataset)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_string(row_ix, 2, &tr.dataset_key)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 3, tr.robust_score, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 4, tr.pnl, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 5, tr.pnl_pct, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 6, tr.sharpe, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 7, tr.max_dd, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 8, tr.trades, &num3_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 9, tr.exposure, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 10, tr.cross_score, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 11, tr.cross_pnl_positive_frac, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 12, tr.psr, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_number_with_format(row_ix, 13, tr.dsr, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                by_asset
                    .write_string(row_ix, 14, tr.ts.as_deref().unwrap_or(""))
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
        }
    }

    {
        let matrix = workbook.add_worksheet();
        matrix
            .set_name("Asset Matrix")
            .map_err(|e: XlsxError| e.to_string())?;
        matrix
            .merge_range(
                0,
                0,
                0,
                10,
                "Per-asset PnL% matrix (median by dataset)",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let mut datasets: BTreeSet<String> = BTreeSet::new();
        for g in &meta.groups {
            for tr in &g.train_rows {
                datasets.insert(tr.dataset.clone());
            }
        }

        if datasets.is_empty() {
            matrix
                .write_string(2, 0, "No training rows available.")
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            matrix
                .write_string_with_format(2, 0, "Rank", &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            matrix
                .write_string_with_format(2, 1, "Meta score", &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            matrix
                .write_string_with_format(2, 2, "Final score", &header_fmt)
                .map_err(|e: XlsxError| e.to_string())?;

            for (i, ds) in datasets.iter().enumerate() {
                matrix
                    .write_string_with_format(2, (3 + i) as u16, ds, &header_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                let _ = matrix.set_column_width((3 + i) as u16, 12.0);
            }
            let _ = matrix.set_column_width(0, 6.0);
            let _ = matrix.set_column_width(1, 11.0);
            let _ = matrix.set_column_width(2, 11.0);
            matrix
                .set_freeze_panes(3, 3)
                .map_err(|e: XlsxError| e.to_string())?;

            for (i, g) in meta.groups.iter().enumerate() {
                let r = 3 + i as u32;
                let final_score = g.final_score.unwrap_or(g.meta_score);
                let by_ds = median_train_pnl_pct_by_dataset(&g.train_rows);

                matrix
                    .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                matrix
                    .write_number_with_format(r, 1, g.meta_score, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                matrix
                    .write_number_with_format(r, 2, final_score, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;

                for (j, ds) in datasets.iter().enumerate() {
                    if let Some(v) = by_ds.get(ds) {
                        matrix
                            .write_number_with_format(r, (3 + j) as u16, *v, &num6_fmt)
                            .map_err(|e: XlsxError| e.to_string())?;
                    } else {
                        matrix
                            .write_string(r, (3 + j) as u16, "-")
                            .map_err(|e: XlsxError| e.to_string())?;
                    }
                }
            }
        }
    }

    {
        let holdout = workbook.add_worksheet();
        holdout
            .set_name("Holdout Rows")
            .map_err(|e: XlsxError| e.to_string())?;
        holdout
            .merge_range(
                0,
                0,
                0,
                8,
                "Per-asset holdout rows by meta group",
                &title_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;

        let headers: [&str; 9] = [
            "Group rank",
            "Dataset",
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
        let widths: [f64; 9] = [10.0, 18.0, 8.0, 10.0, 10.0, 10.0, 11.0, 10.0, 70.0];
        for (c, w) in widths.iter().enumerate() {
            let _ = holdout.set_column_width(c as u16, *w);
        }

        let mut row_ix = header_row + 1;
        for (i, g) in meta.groups.iter().enumerate() {
            let params_json = serde_json::to_string(&g.params).unwrap_or_default();
            for hr in &g.holdout_rows {
                holdout
                    .write_number_with_format(row_ix, 0, (i + 1) as f64, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_string(row_ix, 1, &hr.dataset)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 2, if hr.pass { 1.0 } else { 0.0 }, &int_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 3, hr.score_median, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 4, hr.score_p10, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 5, hr.sharpe_median, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 6, hr.max_dd_worst, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_number_with_format(row_ix, 7, hr.pnl_median, &num6_fmt)
                    .map_err(|e: XlsxError| e.to_string())?;
                holdout
                    .write_string(row_ix, 8, &params_json)
                    .map_err(|e: XlsxError| e.to_string())?;
                row_ix += 1;
            }
        }
        if row_ix == header_row + 1 {
            holdout
                .write_string(row_ix, 0, "No holdout rows were saved for this run.")
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    {
        let dist = workbook.add_worksheet();
        dist.set_name("Distributions")
            .map_err(|e: XlsxError| e.to_string())?;
        dist.merge_range(0, 0, 0, 13, "Meta metric distributions", &title_fmt)
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
            &num6_fmt,
            &int_fmt,
        )?;
        let h4 = write_histogram_table(
            dist,
            2,
            9,
            "MaxDD p90",
            &dd_p90s,
            &header_fmt,
            &num6_fmt,
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

        insert_hist_chart(dist, &h1, "Final score distribution", 2, 12)?;
        insert_hist_chart(dist, &h2, "Robust p10 distribution", 20, 12)?;
        insert_hist_chart(dist, &h3, "Pass rate distribution", 38, 12)?;
        insert_hist_chart(dist, &h4, "MaxDD p90 distribution", 56, 12)?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e: rust_xlsxwriter::XlsxError| e.to_string())
}

fn median_train_pnl_pct_by_dataset(rows: &[TrainDatasetRow]) -> BTreeMap<String, f64> {
    let mut map: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in rows {
        map.entry(r.dataset.clone()).or_default().push(r.pnl_pct);
    }

    let mut out: BTreeMap<String, f64> = BTreeMap::new();
    for (k, mut vs) in map {
        vs.sort_by(|a, b| a.total_cmp(b));
        out.insert(k, quantile(&vs, 0.5));
    }
    out
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
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
) -> Result<Option<(u32, u32, u16, u16)>, String> {
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

    Ok(Some((row0 + 1, row0 + hist.len() as u32, col0, col0 + 1)))
}

fn insert_hist_chart(
    worksheet: &mut Worksheet,
    hist_ref: &Option<(u32, u32, u16, u16)>,
    title: &str,
    row: u32,
    col: u16,
) -> Result<(), String> {
    use rust_xlsxwriter::XlsxError;

    let Some((rmin, rmax, cbin, ccnt)) = hist_ref else {
        return Ok(());
    };

    let mut chart = Chart::new(ChartType::Column);
    chart
        .add_series()
        .set_categories(("Distributions", *rmin, *cbin, *rmax, *cbin))
        .set_values(("Distributions", *rmin, *ccnt, *rmax, *ccnt));
    chart.title().set_name(title);
    chart.x_axis().set_name("Bin center");
    chart.y_axis().set_name("Count");
    chart.legend().set_hidden();
    worksheet
        .insert_chart_with_offset(row, col, &chart, 10, 10)
        .map_err(|e: XlsxError| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ta_app_core::{
        HoldoutAggregate, HoldoutDatasetRow, HoldoutSummary, MetaCandidateAggregate,
        TrainDatasetRow,
    };

    #[test]
    fn meta_report_xlsx_smoke() {
        let params = json!({
            "fast_ma_type": "sma",
            "slow_ma_type": "ema",
            "fast_len": 12,
            "slow_len": 48
        });

        let meta = MetaSelectionResult {
            runtime_ms: 42,
            strategy_kind: "DoubleMa".to_string(),
            timeframe: Some("1m".to_string()),
            artifacts_considered: 3,
            artifacts_used: 3,
            candidates_considered: 12,
            groups: vec![MetaCandidateAggregate {
                params: params.clone(),
                key: "k1".to_string(),
                ma_source: "close".to_string(),
                fast_ma_params: None,
                slow_ma_params: None,
                representatives: vec![params],
                samples: 3,
                datasets_covered: 3,
                pass_rate: 1.0,
                robust_score_median: 0.42,
                robust_score_p10: 0.21,
                tail_gap: 0.21,
                max_dd_p90: 0.18,
                cross_score_median: 0.17,
                psr_median: 0.63,
                dsr_median: 0.51,
                meta_score: 0.33,
                final_score: Some(0.36),
                holdout: Some(HoldoutAggregate {
                    datasets: 2,
                    pass_rate: 0.5,
                    score_median: 0.12,
                    score_p10: -0.05,
                    sharpe_median: 0.22,
                    max_dd_p90: 0.30,
                    pnl_median: 0.08,
                }),
                train_rows: vec![TrainDatasetRow {
                    dataset: "AAPL".to_string(),
                    dataset_key: "csv:aapl.csv".to_string(),
                    robust_score: 0.42,
                    pnl: 0.12,
                    pnl_pct: 0.12,
                    sharpe: 0.8,
                    max_dd: 0.18,
                    trades: 12.0,
                    exposure: 0.55,
                    cross_score: 0.17,
                    cross_pnl_positive_frac: 0.66,
                    psr: 0.63,
                    dsr: 0.51,
                    ts: Some("2026-02-17T00:00:00Z".to_string()),
                }],
                holdout_rows: vec![HoldoutDatasetRow {
                    dataset: "MSFT".to_string(),
                    score_median: 0.10,
                    score_p10: -0.07,
                    sharpe_median: 0.2,
                    max_dd_worst: 0.35,
                    pnl_median: 0.05,
                    pass: true,
                }],
            }],
            warnings: vec!["smoke".to_string()],
            holdout: Some(HoldoutSummary {
                datasets: 2,
                skipped: 0,
            }),
        };

        let bytes =
            build_meta_report_xlsx_bytes("2026-02-17T00:00:00Z", &meta).expect("meta report xlsx");
        assert!(bytes.len() > 1024);
    }
}
