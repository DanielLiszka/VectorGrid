use ta_app_core::MetaSelectionResult;

pub(crate) fn build_meta_report_xlsx_bytes(
    ts: &str,
    meta: &MetaSelectionResult,
) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, XlsxError};

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .set_name("Meta")
        .map_err(|e: XlsxError| e.to_string())?;

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

    let num6_fmt = Format::new().set_num_format("0.000000");
    let num3_fmt = Format::new().set_num_format("0.000");
    let pct_fmt = Format::new().set_num_format("0.0%");
    let int_fmt = Format::new().set_num_format("0");

    worksheet
        .merge_range(0, 0, 0, 24, "VectorTA Meta Report", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(1, 0, "ts")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 1, ts)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(2, 0, "strategy_kind")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(2, 1, &meta.strategy_kind)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(3, 0, "timeframe")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(3, 1, meta.timeframe.as_deref().unwrap_or(""))
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(h) = meta.holdout.as_ref() {
        worksheet
            .write_string(4, 0, "holdout_datasets")
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(4, 1, h.datasets as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(4, 3, "holdout_skipped")
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(4, 4, h.skipped as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    worksheet
        .write_string(1, 3, "artifacts_used")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(1, 4, meta.artifacts_used as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(2, 3, "candidates_considered")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(2, 4, meta.candidates_considered as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(3, 3, "runtime_ms")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(3, 4, meta.runtime_ms as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    if meta.candidates_considered >= 1_000 || meta.artifacts_used >= 100 {
        worksheet
            .write_string(1, 6, "selection_bias_warning")
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(
                1,
                7,
                "Large search space. Interpret winners with holdout context and lower-tail robustness metrics.",
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let header_row: u32 = 5;
    let headers: [&str; 25] = [
        "Rank",
        "Samples",
        "Pass rate",
        "Meta score",
        "Robust p10",
        "Robust med",
        "MaxDD p90",
        "Cross med",
        "PSR med",
        "DSR med",
        "H pass",
        "H score p10",
        "H score med",
        "H DD p90",
        "Fast MA",
        "Fast Len",
        "Slow MA",
        "Slow Len",
        "HT amp",
        "HT atr",
        "HT ch_dev",
        "MACZ fast",
        "MACZ slow",
        "OTT period",
        "OTT pct",
    ];
    for (c, h) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(header_row, c as u16, *h, &header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    for (i, row) in meta.groups.iter().enumerate() {
        let r = header_row + 1 + (i as u32);

        let p = row.params.as_object();
        let fast_ma = p
            .and_then(|p| p.get("fast_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let slow_ma = p
            .and_then(|p| p.get("slow_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let fast_len = p
            .and_then(|p| p.get("fast_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let slow_len = p
            .and_then(|p| p.get("slow_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let ht = p.and_then(|p| p.get("halftrend").and_then(|v| v.as_object()));
        let ht_amp = ht
            .and_then(|h| h.get("amplitude").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let ht_atr = ht
            .and_then(|h| h.get("atr_period").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let ht_dev = ht
            .and_then(|h| h.get("channel_deviation").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        let macz = p.and_then(|p| p.get("macz").and_then(|v| v.as_object()));
        let macz_fast = macz
            .and_then(|m| m.get("fast_length").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let macz_slow = macz
            .and_then(|m| m.get("slow_length").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let ott = p.and_then(|p| p.get("ott").and_then(|v| v.as_object()));
        let ott_period = ott
            .and_then(|o| o.get("period").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let ott_pct = ott
            .and_then(|o| o.get("percent").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        worksheet
            .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 1, row.samples as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 2, row.pass_rate, &pct_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 3, row.meta_score, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 4, row.robust_score_p10, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 5, row.robust_score_median, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 6, row.max_dd_p90, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 7, row.cross_score_median, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 8, row.psr_median, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 9, row.dsr_median, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        if let Some(h) = row.holdout.as_ref() {
            worksheet
                .write_number_with_format(r, 10, h.pass_rate, &pct_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 11, h.score_p10, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 12, h.score_median, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 13, h.max_dd_p90, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            worksheet
                .write_string(r, 10, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string(r, 11, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string(r, 12, "-")
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string(r, 13, "-")
                .map_err(|e: XlsxError| e.to_string())?;
        }

        worksheet
            .write_string(r, 14, fast_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 15, fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 16, slow_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 17, slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 18, ht_amp as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 19, ht_atr as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 20, ht_dev, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 21, macz_fast as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 22, macz_slow as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 23, ott_period as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 24, ott_pct, &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e: XlsxError| e.to_string())
}
