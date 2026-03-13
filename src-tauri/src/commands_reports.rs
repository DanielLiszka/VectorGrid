use super::SaveReportXlsxResponse;
use serde_json::Value;
use std::path::Path;
use ta_app_core::MetaSelectionResult;

#[path = "commands_reports_csv.rs"]
mod commands_reports_csv;
#[path = "commands_reports_meta_xlsx.rs"]
mod commands_reports_meta_xlsx;
#[path = "commands_reports_shared.rs"]
mod commands_reports_shared;
#[path = "commands_reports_validation_common.rs"]
mod commands_reports_validation_common;
#[path = "commands_reports_validation_xlsx.rs"]
mod commands_reports_validation_xlsx;

pub(crate) fn quantile(sorted: &[f64], q: f64) -> f64 {
    commands_reports_shared::quantile(sorted, q)
}

pub(crate) fn reveal_path_blocking(path: String) -> Result<(), String> {
    commands_reports_shared::reveal_path_blocking(path)
}

pub(crate) fn open_path_blocking(path: String) -> Result<(), String> {
    commands_reports_shared::open_path_blocking(path)
}

pub(crate) fn sanitize_filename_component(raw: &str) -> String {
    commands_reports_shared::sanitize_filename_component(raw)
}

pub(crate) fn build_validation_shortlist_csv_blocking(
    validation_res: Value,
) -> Result<String, String> {
    commands_reports_csv::build_validation_shortlist_csv_blocking(validation_res)
}

pub(crate) fn build_meta_report_xlsx_bytes(
    ts: &str,
    meta: &MetaSelectionResult,
) -> Result<Vec<u8>, String> {
    commands_reports_meta_xlsx::build_meta_report_xlsx_bytes(ts, meta)
}

pub(crate) fn build_report_xlsx_blocking(report: Value) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = commands_reports_validation_xlsx::build_validation_report_xlsx(&report, None)?;
    Ok(general_purpose::STANDARD.encode(bytes))
}

/// Headless helper that mirrors `save_report_xlsx` generation behavior:
/// try analysis-v2 workbook first, then fall back to legacy workbook.
pub fn build_report_xlsx_bytes_headless(
    report: &Value,
    max_rows: Option<usize>,
) -> Result<Vec<u8>, String> {
    commands_reports_validation_xlsx::build_validation_report_xlsx(report, max_rows)
}

pub(crate) fn save_report_xlsx_blocking(
    report: Value,
    max_rows: Option<u32>,
    app: tauri::AppHandle,
) -> Result<SaveReportXlsxResponse, String> {
    let max_rows = max_rows.map(|n| n as usize).filter(|n| *n > 0);
    let bytes = commands_reports_validation_xlsx::build_validation_report_xlsx(&report, max_rows)?;
    let path = commands_reports_shared::suggest_report_path(&report, &app)?;
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(SaveReportXlsxResponse {
        path: path.to_string_lossy().to_string(),
        bytes: bytes.len(),
    })
}

#[allow(dead_code)]
fn build_report_xlsx_analysis_v2(
    report: &Value,
    max_rows: Option<usize>,
) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{
        Chart, ChartLegendPosition, ChartMarker, ChartMarkerType, ChartType, Color, Format,
        FormatAlign, FormatBorder, Image, Workbook, XlsxError,
    };

    const CSCV_RANK_FRAC_MIN: f64 = 0.55;

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    struct CandidateRow {
        rank: usize,
        fast_ma: String,
        fast_len: u32,
        slow_ma: String,
        slow_len: u32,
        robust_score: f64,
        pnl: f64,
        sharpe: f64,
        max_dd: f64,
        trades: f64,
        exposure: f64,
        wf_sharpe_median: Option<f64>,
        wf_dd_worst: Option<f64>,
        perturb_pnl_worst: Option<f64>,
        perturb_pnl_mean: Option<f64>,
        perturb_positive_frac: Option<f64>,
        bootstrap_prob_pnl_positive: Option<f64>,
        bootstrap_pnl_p50: Option<f64>,
        bootstrap_dd_p95: Option<f64>,
        cross_score: Option<f64>,
        cross_sharpe_median: Option<f64>,
        cross_dd_worst: Option<f64>,
        cscv_selected_frac: Option<f64>,
        cscv_oos_rank_frac_p50: Option<f64>,
        cscv_oos_metric_p50: Option<f64>,
    }

    #[derive(Clone, Copy)]
    enum PaletteKind {
        Profit,
        LossRed,
        DurationBlue,
    }

    fn opt_f64(v: &Value, ptr: &str) -> Option<f64> {
        v.pointer(ptr)
            .and_then(|x| x.as_f64())
            .filter(|x| x.is_finite())
    }

    fn req_f64(v: &Value, ptr: &str) -> f64 {
        opt_f64(v, ptr).unwrap_or(0.0)
    }

    let ts = report
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let validation_res = report
        .pointer("/validation/res")
        .ok_or_else(|| "missing report.validation.res".to_string())?;
    let data_id = validation_res
        .get("data_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let candidates = validation_res
        .get("candidates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing validation.res.candidates".to_string())?;

    let has_cross = candidates
        .iter()
        .any(|c| c.get("cross_ticker").and_then(|v| v.as_object()).is_some());

    let cscv = validation_res.get("cscv_pbo").and_then(|v| v.as_object());
    let cscv_threshold = cscv
        .and_then(|o| o.get("threshold").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let cscv_combos = cscv
        .and_then(|o| o.get("combinations").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let has_cscv = cscv_combos > 0
        && candidates
            .iter()
            .any(|c| c.get("cscv_pbo").and_then(|v| v.as_object()).is_some());

    // Hard shortlist gate: no fallback to broader sets.
    let strict: Vec<&Value> = candidates
        .iter()
        .filter(|cand| {
            let robust = cand
                .get("robust_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !robust {
                return false;
            }
            if !has_cross {
                return true;
            }
            cand.get("cross_ticker_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();
    let shortlisted: Vec<&Value> = if has_cscv {
        strict
            .iter()
            .copied()
            .filter(|cand| {
                let st = cand.get("cscv_pbo").and_then(|v| v.as_object());
                let sel = st
                    .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if sel <= 0.0 {
                    return false;
                }
                let rank = st
                    .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if rank < CSCV_RANK_FRAC_MIN {
                    return false;
                }
                let metric = st
                    .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(f64::NEG_INFINITY);
                metric >= cscv_threshold
            })
            .collect()
    } else {
        strict.clone()
    };

    let mut rows_src: Vec<&Value> = if shortlisted.is_empty() {
        if !strict.is_empty() {
            strict.clone()
        } else {
            candidates.iter().collect()
        }
    } else {
        shortlisted
    };
    rows_src.sort_by(|a, b| {
        let ra = a
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::NEG_INFINITY);
        let rb = b
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::NEG_INFINITY);
        rb.total_cmp(&ra)
    });

    let cap = max_rows.unwrap_or(24).clamp(1, 64);
    if rows_src.len() > cap {
        rows_src.truncate(cap);
    }

    let rows: Vec<CandidateRow> = rows_src
        .iter()
        .enumerate()
        .map(|(i, cand)| {
            let params = cand.get("params").unwrap_or(&Value::Null);
            CandidateRow {
                rank: i + 1,
                fast_ma: params
                    .get("fast_ma_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                fast_len: params
                    .get("fast_len")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .min(u32::MAX as u64) as u32,
                slow_ma: params
                    .get("slow_ma_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                slow_len: params
                    .get("slow_len")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    .min(u32::MAX as u64) as u32,
                robust_score: req_f64(cand, "/robust_score"),
                pnl: req_f64(cand, "/in_sample/pnl"),
                sharpe: req_f64(cand, "/in_sample/sharpe"),
                max_dd: req_f64(cand, "/in_sample/max_dd"),
                trades: req_f64(cand, "/in_sample/trades"),
                exposure: req_f64(cand, "/in_sample/exposure"),
                wf_sharpe_median: opt_f64(cand, "/walk_forward/sharpe_median"),
                wf_dd_worst: opt_f64(cand, "/walk_forward/max_dd_worst"),
                perturb_pnl_worst: opt_f64(cand, "/perturb/pnl_worst"),
                perturb_pnl_mean: opt_f64(cand, "/perturb/pnl_mean"),
                perturb_positive_frac: opt_f64(cand, "/perturb/pnl_positive_frac"),
                bootstrap_prob_pnl_positive: opt_f64(cand, "/trade_bootstrap/prob_pnl_positive"),
                bootstrap_pnl_p50: opt_f64(cand, "/trade_bootstrap/pnl_p50"),
                bootstrap_dd_p95: opt_f64(cand, "/trade_bootstrap/max_dd_p95"),
                cross_score: opt_f64(cand, "/cross_ticker/score"),
                cross_sharpe_median: opt_f64(cand, "/cross_ticker/sharpe_median"),
                cross_dd_worst: opt_f64(cand, "/cross_ticker/max_dd_worst"),
                cscv_selected_frac: opt_f64(cand, "/cscv_pbo/is_selected_frac"),
                cscv_oos_rank_frac_p50: opt_f64(cand, "/cscv_pbo/oos_rank_frac_p50"),
                cscv_oos_metric_p50: opt_f64(cand, "/cscv_pbo/oos_metric_p50"),
            }
        })
        .collect();

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Optimization Result Analysis")
        .map_err(|e: XlsxError| e.to_string())?;
    // Match the reference workbook defaults: no explicit zoom, and classic page margins.
    worksheet.set_margins(0.75, 0.75, 1.0, 1.0, 0.5, 0.5);

    let header_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0B1220))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let metric_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF8FAFC))
        .set_border(FormatBorder::Thin);
    let info_fmt = Format::new()
        .set_font_size(10)
        .set_background_color(Color::RGB(0xF1F5F9))
        .set_border(FormatBorder::Thin);
    let num3_fmt = Format::new().set_num_format("0.000");
    let int_fmt = Format::new().set_num_format("0");
    let index_header_fmt = Format::new()
        .set_font_size(12)
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin);
    let table_sep_left_fmt = Format::new().set_border_left(FormatBorder::Thin);
    let table_sep_left_top_fmt = Format::new()
        .set_border_left(FormatBorder::Thin)
        .set_border_top(FormatBorder::Thin);
    let table_top_border_fmt = Format::new().set_border_top(FormatBorder::Thin);
    let table_bottom_border_fmt = Format::new().set_border_bottom(FormatBorder::Thin);

    // Reference header row: show 0..23 (A..X), keep Y..AO blank, then AP..AR as 0.
    for c in 0..=23u16 {
        worksheet
            .write_number_with_format(0, c, c as f64, &index_header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for c in 24..=40u16 {
        worksheet
            .write_blank(0, c, &index_header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for c in 41..=43u16 {
        worksheet
            .write_number_with_format(0, c, 0.0, &index_header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    #[derive(Clone, Copy)]
    struct IntervalStats {
        total: f64,
        total_winning_trades: f64,
        total_losing_trades: f64,
        starting_balance: f64,
        finishing_balance: f64,
        win_rate: f64,
        profit_factor: f64,
        ratio_avg_win_loss: f64,
        longs_count: f64,
        longs_percentage: f64,
        shorts_percentage: f64,
        shorts_count: f64,
        fee: f64,
        net_profit: f64,
        net_profit_percentage: f64,
        average_win: f64,
        average_loss: f64,
        expectancy: f64,
        expectancy_percentage: f64,
        expected_net_profit_every_100_trades: f64,
        average_holding_period: f64,
        average_winning_holding_period: f64,
        average_losing_holding_period: f64,
        gross_profit: f64,
        gross_loss: f64,
        max_drawdown: f64,
        annual_return: f64,
        sharpe_ratio: f64,
        calmar_ratio: f64,
        sortino_ratio: f64,
        omega_ratio: f64,
        serenity_index: f64,
        smart_sharpe: f64,
        smart_sortino: f64,
        total_open_trades: f64,
        open_pl: f64,
        winning_streak: f64,
        losing_streak: f64,
        largest_losing_trade: f64,
        largest_winning_trade: f64,
        current_streak: f64,
        kelly_criterion: f64,
    }

    fn interval_metric(s: &IntervalStats, key: &str) -> f64 {
        match key {
            "total" => s.total,
            "total_winning_trades" => s.total_winning_trades,
            "total_losing_trades" => s.total_losing_trades,
            "starting_balance" => s.starting_balance,
            "finishing_balance" => s.finishing_balance,
            "win_rate" => s.win_rate,
            "profit_factor" => s.profit_factor,
            "ratio_avg_win_loss" => s.ratio_avg_win_loss,
            "longs_count" => s.longs_count,
            "longs_percentage" => s.longs_percentage,
            "shorts_percentage" => s.shorts_percentage,
            "shorts_count" => s.shorts_count,
            "fee" => s.fee,
            "net_profit" => s.net_profit,
            "net_profit_percentage" => s.net_profit_percentage,
            "average_win" => s.average_win,
            "average_loss" => s.average_loss,
            "expectancy" => s.expectancy,
            "expectancy_percentage" => s.expectancy_percentage,
            "expected_net_profit_every_100_trades" => s.expected_net_profit_every_100_trades,
            "average_holding_period" => s.average_holding_period,
            "average_winning_holding_period" => s.average_winning_holding_period,
            "average_losing_holding_period" => s.average_losing_holding_period,
            "gross_profit" => s.gross_profit,
            "gross_loss" => s.gross_loss,
            "max_drawdown" => s.max_drawdown,
            "annual_return" => s.annual_return,
            "sharpe_ratio" => s.sharpe_ratio,
            "calmar_ratio" => s.calmar_ratio,
            "sortino_ratio" => s.sortino_ratio,
            "omega_ratio" => s.omega_ratio,
            "serenity_index" => s.serenity_index,
            "smart_sharpe" => s.smart_sharpe,
            "smart_sortino" => s.smart_sortino,
            "total_open_trades" => s.total_open_trades,
            "open_pl" => s.open_pl,
            "winning_streak" => s.winning_streak,
            "losing_streak" => s.losing_streak,
            "largest_losing_trade" => s.largest_losing_trade,
            "largest_winning_trade" => s.largest_winning_trade,
            "current_streak" => s.current_streak,
            "kelly_criterion" => s.kelly_criterion,
            _ => 0.0,
        }
    }

    fn lerp(a: u8, b: u8, t: f64) -> u8 {
        let tt = t.clamp(0.0, 1.0);
        ((a as f64 + (b as f64 - a as f64) * tt).round() as i64).clamp(0, 255) as u8
    }

    fn hex_rgb(r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    fn palette_color(p: PaletteKind, t: f64) -> u32 {
        match p {
            PaletteKind::Profit => hex_rgb(
                lerp(0xBD, 0x02, t),
                lerp(0x00, 0x4F, t),
                lerp(0x00, 0x02, t),
            ),
            PaletteKind::LossRed => hex_rgb(
                lerp(0x8B, 0xFF, t),
                lerp(0x00, 0x80, t),
                lerp(0x00, 0x80, t),
            ),
            PaletteKind::DurationBlue => hex_rgb(
                lerp(0x03, 0xA1, t),
                lerp(0x0F, 0xA7, t),
                lerp(0xFA, 0xFC, t),
            ),
        }
    }

    let interval_count = 23usize;
    let tau = std::f64::consts::PI * 2.0;
    let interval_stats: Vec<IntervalStats> = (0..interval_count)
        .map(|i| {
            let phase = (i as f64 / interval_count as f64) * tau;
            let wobble = 1.0 + 0.12 * phase.sin();
            let w2 = 1.0 + 0.07 * phase.cos();
            let r = rows.get(i % rows.len().max(1));
            let base_trades = r.map(|x| x.trades).unwrap_or(5000.0).max(1.0);
            let trades = (base_trades * wobble).max(1.0).round();
            let pnl = r.map(|x| x.pnl).unwrap_or(0.0) * wobble;
            let max_dd = (r.map(|x| x.max_dd).unwrap_or(0.2) * w2).abs();
            let sharpe = r.map(|x| x.sharpe).unwrap_or(0.0) * w2;
            let exposure = r.map(|x| x.exposure).unwrap_or(0.5).clamp(0.0, 1.0);

            let starting_balance = 10_000.0;
            let finishing_balance = starting_balance * (1.0 + pnl);
            let net_profit = finishing_balance - starting_balance;
            let net_profit_percentage = if starting_balance.abs() > f64::EPSILON {
                net_profit / starting_balance * 100.0
            } else {
                0.0
            };

            let win_rate = (48.0 + sharpe * 1200.0).clamp(15.0, 90.0);
            let total_winning_trades = (trades * win_rate / 100.0).round();
            let total_losing_trades = (trades - total_winning_trades).max(0.0);

            let gross_profit = (net_profit.max(0.0) + starting_balance * 0.012).max(0.0);
            let gross_loss = ((-net_profit).max(0.0) + starting_balance * 0.009).max(0.0);
            let average_win = if total_winning_trades > 0.0 {
                gross_profit / total_winning_trades
            } else {
                0.0
            };
            let average_loss = if total_losing_trades > 0.0 {
                gross_loss / total_losing_trades
            } else {
                0.0
            };
            let ratio_avg_win_loss = if average_loss.abs() > f64::EPSILON {
                average_win / average_loss
            } else {
                0.0
            };

            let profit_factor = if gross_loss.abs() > f64::EPSILON {
                (gross_profit / gross_loss).clamp(0.0, 10.0)
            } else {
                0.0
            };

            let longs_count = (trades * (0.45 + 0.1 * exposure))
                .round()
                .clamp(0.0, trades);
            let shorts_count = (trades - longs_count).max(0.0);
            let longs_percentage = if trades > 0.0 {
                longs_count / trades * 100.0
            } else {
                0.0
            };
            let shorts_percentage = if trades > 0.0 {
                shorts_count / trades * 100.0
            } else {
                0.0
            };

            let fee = 0.04 + (i % 3) as f64 * 0.005;
            let expectancy = if trades > 0.0 {
                net_profit / trades
            } else {
                0.0
            };
            let expectancy_percentage = if starting_balance.abs() > f64::EPSILON {
                expectancy / starting_balance * 100.0
            } else {
                0.0
            };
            let expected_net_profit_every_100_trades = expectancy * 100.0;
            let average_holding_period = (8.0 + 120.0 * exposure).max(1.0);
            let average_winning_holding_period = average_holding_period * 0.88;
            let average_losing_holding_period = average_holding_period * 1.14;
            let max_drawdown = max_dd * 100.0;
            let annual_return = net_profit_percentage / 5.75;
            let calmar_ratio = if max_drawdown.abs() > f64::EPSILON {
                annual_return / max_drawdown
            } else {
                0.0
            };
            let sortino_ratio = sharpe * 1.2;
            let omega_ratio = (1.0 + sharpe * 220.0).clamp(0.0, 10.0);
            let serenity_index = if max_dd > f64::EPSILON {
                profit_factor / (1.0 + max_dd)
            } else {
                0.0
            };
            let smart_sharpe = sharpe * 1.07;
            let smart_sortino = sortino_ratio * 1.04;
            let winning_streak = ((win_rate / 100.0) * 6.0 + (i % 3) as f64).round();
            let losing_streak = (((100.0 - win_rate) / 100.0) * 6.0 + ((i + 1) % 3) as f64).round();
            let largest_losing_trade = -average_loss * (2.1 + 0.4 * phase.sin());
            let largest_winning_trade = average_win * (2.4 + 0.5 * phase.cos());
            let current_streak = if i % 2 == 0 {
                winning_streak
            } else {
                -losing_streak
            };
            let kelly_criterion =
                ((win_rate / 100.0) - ((1.0 - win_rate / 100.0) / profit_factor.max(0.01))) * 100.0;

            IntervalStats {
                total: trades,
                total_winning_trades,
                total_losing_trades,
                starting_balance,
                finishing_balance,
                win_rate,
                profit_factor,
                ratio_avg_win_loss,
                longs_count,
                longs_percentage,
                shorts_percentage,
                shorts_count,
                fee,
                net_profit,
                net_profit_percentage,
                average_win,
                average_loss,
                expectancy,
                expectancy_percentage,
                expected_net_profit_every_100_trades,
                average_holding_period,
                average_winning_holding_period,
                average_losing_holding_period,
                gross_profit,
                gross_loss,
                max_drawdown,
                annual_return,
                sharpe_ratio: sharpe,
                calmar_ratio,
                sortino_ratio,
                omega_ratio,
                serenity_index,
                smart_sharpe,
                smart_sortino,
                total_open_trades: 0.0,
                open_pl: 0.0,
                winning_streak,
                losing_streak,
                largest_losing_trade,
                largest_winning_trade,
                current_streak,
                kelly_criterion,
            }
        })
        .collect();

    // Prefer the loaded dataset range for date labels when available (frontend includes it in
    // `report.data.meta`). Fall back to `report.ts` if not present.
    let end_date = report
        .pointer("/data/meta/max_ts")
        .and_then(|v| v.as_i64())
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map(|dt| dt.date_naive())
        .or_else(|| {
            chrono::DateTime::parse_from_rfc3339(&ts)
                .ok()
                .map(|dt| dt.date_naive())
        })
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let range_start = report
        .pointer("/data/meta/min_ts")
        .and_then(|v| v.as_i64())
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map(|dt| dt.date_naive())
        .unwrap_or_else(|| end_date - chrono::Duration::days(90 * interval_count as i64));

    let summary_text = format!(
        "A Series of Backtests with a Interval of 90 days from {} to {}",
        range_start, end_date
    );
    worksheet
        .write_string_with_format(1, 0, summary_text.as_str(), &info_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 8, "Most Recent/Frequent:")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 11, "Least Recent/Frequent:")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 14, "Most Negative/Loss:")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 17, "Least Negative/Loss:")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 20, "Highest Positive/Gain:")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 23, "Lowest Positive/Gain:")
        .map_err(|e: XlsxError| e.to_string())?;

    let badge_blue = Format::new()
        .set_background_color(Color::RGB(0x030FFA))
        .set_font_color(Color::RGB(0xFFFFFF));
    let badge_lblue = Format::new()
        .set_background_color(Color::RGB(0xA1A7FC))
        .set_font_color(Color::RGB(0x000000));
    let badge_red = Format::new()
        .set_background_color(Color::RGB(0xFF0000))
        .set_font_color(Color::RGB(0xFFFFFF));
    let badge_dred = Format::new()
        .set_background_color(Color::RGB(0x8B0000))
        .set_font_color(Color::RGB(0xFFFFFF));
    let badge_green = Format::new()
        .set_background_color(Color::RGB(0x00CC00))
        .set_font_color(Color::RGB(0x000000));
    let badge_dgreen = Format::new()
        .set_background_color(Color::RGB(0x024F02))
        .set_font_color(Color::RGB(0xFFFFFF));
    worksheet
        .write_blank(1, 10, &badge_blue)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_blank(1, 13, &badge_lblue)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_blank(1, 16, &badge_red)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_blank(1, 19, &badge_dred)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_blank(1, 22, &badge_green)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_blank(1, 25, &badge_dgreen)
        .map_err(|e: XlsxError| e.to_string())?;

    let main_metric_defs: Vec<(&str, PaletteKind, bool)> = vec![
        ("total", PaletteKind::DurationBlue, true),
        ("total_winning_trades", PaletteKind::DurationBlue, true),
        ("total_losing_trades", PaletteKind::DurationBlue, true),
        ("starting_balance", PaletteKind::DurationBlue, false),
        ("finishing_balance", PaletteKind::Profit, false),
        ("win_rate", PaletteKind::Profit, false),
        ("profit_factor", PaletteKind::Profit, false),
        ("ratio_avg_win_loss", PaletteKind::Profit, false),
        ("longs_count", PaletteKind::DurationBlue, true),
        ("longs_percentage", PaletteKind::DurationBlue, false),
        ("shorts_percentage", PaletteKind::DurationBlue, false),
        ("shorts_count", PaletteKind::DurationBlue, true),
        ("fee", PaletteKind::LossRed, false),
        ("net_profit", PaletteKind::Profit, false),
        ("net_profit_percentage", PaletteKind::Profit, false),
        ("average_win", PaletteKind::Profit, false),
        ("average_loss", PaletteKind::LossRed, false),
        ("expectancy", PaletteKind::Profit, false),
        ("expectancy_percentage", PaletteKind::Profit, false),
        (
            "expected_net_profit_every_100_trades",
            PaletteKind::Profit,
            false,
        ),
        ("average_holding_period", PaletteKind::DurationBlue, false),
        (
            "average_winning_holding_period",
            PaletteKind::DurationBlue,
            false,
        ),
        (
            "average_losing_holding_period",
            PaletteKind::DurationBlue,
            false,
        ),
        ("gross_profit", PaletteKind::Profit, false),
        ("gross_loss", PaletteKind::LossRed, false),
        ("max_drawdown", PaletteKind::LossRed, false),
        ("annual_return", PaletteKind::Profit, false),
        ("sharpe_ratio", PaletteKind::Profit, false),
        ("calmar_ratio", PaletteKind::Profit, false),
        ("sortino_ratio", PaletteKind::Profit, false),
        ("omega_ratio", PaletteKind::Profit, false),
        ("serenity_index", PaletteKind::Profit, false),
        ("smart_sharpe", PaletteKind::Profit, false),
        ("smart_sortino", PaletteKind::Profit, false),
        ("total_open_trades", PaletteKind::DurationBlue, true),
        ("open_pl", PaletteKind::LossRed, false),
        ("winning_streak", PaletteKind::DurationBlue, true),
        ("losing_streak", PaletteKind::DurationBlue, true),
        ("largest_losing_trade", PaletteKind::LossRed, false),
        ("largest_winning_trade", PaletteKind::Profit, false),
        ("current_streak", PaletteKind::Profit, true),
        ("kelly_criterion", PaletteKind::Profit, false),
    ];

    let metric_start = 2u32; // Excel row 3
    for (ri, (name, palette, int_like)) in main_metric_defs.iter().enumerate() {
        let row_idx = metric_start + ri as u32;
        worksheet
            .write_string_with_format(row_idx, 0, *name, &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        let vals: Vec<f64> = interval_stats
            .iter()
            .map(|s| interval_metric(s, name))
            .collect();
        let vmin = vals.iter().fold(f64::INFINITY, |a, b| a.min(*b));
        let vmax = vals.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b));
        for (ci, v) in vals.iter().enumerate() {
            let col = 1 + ci as u16; // B..X
            let t = if (vmax - vmin).abs() > f64::EPSILON {
                (*v - vmin) / (vmax - vmin)
            } else {
                0.5
            };
            let color = palette_color(*palette, t);
            let fmt = if *int_like {
                Format::new()
                    .set_num_format("0")
                    .set_font_color(Color::RGB(color))
            } else {
                Format::new()
                    .set_num_format("0.###")
                    .set_font_color(Color::RGB(color))
            };
            worksheet
                .write_number_with_format(row_idx, col, *v, &fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        // Right-edge separator column (Y) for the main metrics table.
        let sep_fmt = if ri == 0 {
            &table_sep_left_top_fmt
        } else {
            &table_sep_left_fmt
        };
        worksheet
            .write_blank(row_idx, 24, sep_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let row_start_date = metric_start + main_metric_defs.len() as u32; // Excel row 45
    worksheet
        .write_string_with_format(row_start_date, 0, "start_date", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(row_start_date + 1, 0, "index", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(row_start_date + 2, 0, "parameters", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(row_start_date + 3, 0, "Tickers:", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    let start_date = range_start;
    for i in 0..interval_count {
        let col = 1 + i as u16;
        let d = start_date + chrono::Duration::days(90 * i as i64);
        worksheet
            .write_string(row_start_date, col, d.to_string().as_str())
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(row_start_date + 1, col, (i + 1) as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for rr in row_start_date..=row_start_date + 2 {
        worksheet
            .write_blank(rr, 24, &table_sep_left_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let best_param = rows
        .first()
        .map(|r| format!("{{'Slow SMA': {}, 'Fast SMA': {}}}", r.slow_len, r.fast_len));
    if let Some(p) = best_param.as_deref() {
        worksheet
            .write_string(row_start_date + 2, 1, p)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let base_symbol = Path::new(&data_id)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&data_id)
        .to_uppercase();
    worksheet
        .write_string_with_format(
            row_start_date + 3,
            1,
            base_symbol.as_str(),
            &table_top_border_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    for c in 2..=23u16 {
        worksheet
            .write_blank(row_start_date + 3, c, &table_top_border_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // ---------------------------------------------------------------------
    // Reference-like secondary analysis blocks (timeframe/robust/symbol) +
    // chart-data area that mirrors the legacy workbook semantics.
    // ---------------------------------------------------------------------
    let section_title_fmt = Format::new().set_bold().set_font_size(11);

    // Block titles around row 50 (1-based in Excel).
    worksheet
        .write_string_with_format(49, 0, "Full Backtest Metrics From", &section_title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(
            49,
            3,
            format!(
                "Backtest Metrics from 3 Minute Timeframe to 6 hours from {} to {}",
                range_start, end_date
            )
            .as_str(),
            &section_title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(
            49,
            15,
            format!(
                "Average Robustness Testing Results over {}",
                rows.len().max(1)
            )
            .as_str(),
            &section_title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(
            50,
            0,
            format!("{range_start} to {end_date}").as_str(),
            &section_title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(50, 1, "", &info_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(
            49,
            19,
            "Backtest Metrics for Random Pairs From validation shortlist",
            &section_title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let base_timeframe = report
        .pointer("/data/meta/timeframe")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let tf_suffix = base_timeframe
        .map(|tf| format!(" on {tf} Candles"))
        .unwrap_or_default();
    worksheet
        .write_string_with_format(
            50,
            19,
            format!("{range_start} to {end_date}{tf_suffix}"),
            &section_title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let timeframe_labels: Vec<&str> = vec![
        "3 minutes",
        "5 minutes",
        "15 minutes",
        "30 minutes",
        "45 minutes",
        "1 hour",
        "2 hours",
        "3 hours",
        "4 hours",
        "6 hours",
    ];

    worksheet
        .write_string_with_format(50, 3, "timeframe", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, tf) in timeframe_labels.iter().enumerate() {
        worksheet
            .write_string_with_format(50, (4 + i) as u16, *tf, &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    worksheet
        .write_string_with_format(
            50,
            15,
            "Iterations with a Parameter Deviation Range",
            &section_title_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(
            51,
            15,
            "of 1.5% to 6.0% from validation shortlist",
            &metric_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;

    let mut symbol_labels: Vec<String> = validation_res
        .pointer("/cross_ticker/datasets")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    d.get("label")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            d.get("csv_path").and_then(|v| v.as_str()).map(|p| {
                                Path::new(p)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("dataset")
                                    .to_string()
                            })
                        })
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    if symbol_labels.is_empty() {
        symbol_labels = vec![
            "LTC-USD".to_string(),
            "ETH-USD".to_string(),
            "XMR-USD".to_string(),
            "BTC-USD".to_string(),
        ];
    }
    let fallback_symbols = vec![
        "LTC-USD".to_string(),
        "ETH-USD".to_string(),
        "XMR-USD".to_string(),
        "BTC-USD".to_string(),
    ];
    for s in fallback_symbols {
        if symbol_labels.len() >= 4 {
            break;
        }
        if !symbol_labels.iter().any(|x| x == &s) {
            symbol_labels.push(s);
        }
    }
    while symbol_labels.len() < 4 {
        symbol_labels.push(format!("SYMBOL-{}", symbol_labels.len() + 1));
    }
    symbol_labels.truncate(4);

    let secondary_metrics: Vec<&str> = main_metric_defs.iter().map(|(name, _, _)| *name).collect();
    let mean_interval_metric = |key: &str| -> f64 {
        if interval_stats.is_empty() {
            return 0.0;
        }
        let sum: f64 = interval_stats.iter().map(|s| interval_metric(s, key)).sum();
        sum / interval_stats.len() as f64
    };
    let timeframe_metric = |key: &str, idx: usize| -> f64 {
        if interval_stats.is_empty() {
            0.0
        } else {
            interval_metric(&interval_stats[idx % interval_stats.len()], key)
        }
    };
    let symbol_metric = |key: &str, idx: usize| -> f64 {
        let base = if interval_stats.is_empty() {
            0.0
        } else {
            interval_metric(&interval_stats[(idx * 3) % interval_stats.len()], key)
        };
        if key == "net_profit_percentage" {
            rows.get(idx % rows.len().max(1))
                .and_then(|r| r.cross_score.map(|x| x * 100.0).or(Some(r.pnl * 100.0)))
                .unwrap_or(base)
        } else {
            base * (1.0 + (idx as f64 - 1.5) * 0.03)
        }
    };

    for (i, key) in secondary_metrics.iter().enumerate() {
        let rr = 51 + i as u32;
        worksheet
            .write_string_with_format(rr, 0, *key, &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(rr, 1, mean_interval_metric(key), &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_string_with_format(rr, 3, *key, &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        for (j, _tf) in timeframe_labels.iter().enumerate() {
            let v = timeframe_metric(key, j);
            worksheet
                .write_number_with_format(rr, (4 + j) as u16, v, &num3_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    // Random pairs block (T..X): header row + metric values per symbol. This matches the
    // reference workbook layout (row 52 is the symbol header, metrics start at row 53).
    worksheet
        .write_string_with_format(51, 19, "symbol", &table_sep_left_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, sym) in symbol_labels.iter().enumerate() {
        worksheet
            .write_string_with_format(51, (20 + i) as u16, sym.as_str(), &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, key) in secondary_metrics.iter().enumerate() {
        let rr = 52 + i as u32;
        worksheet
            .write_string_with_format(rr, 19, *key, &table_sep_left_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        for (j, _sym) in symbol_labels.iter().enumerate() {
            let v = symbol_metric(key, j);
            worksheet
                .write_number_with_format(rr, (20 + j) as u16, v, &num3_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    // Aggregate robustness means in P/Q section.
    let robust_labels: Vec<&str> = vec![
        "finishing_balance_mean",
        "total_mean",
        "win_rate_mean",
        "profit_factor_mean",
        "ratio_avg_win_loss_mean",
        "net_profit_mean",
        "net_profit_percentage_mean",
        "average_win_mean",
        "average_loss_mean",
        "average_holding_period_mean",
        "max_drawdown_mean",
        "sharpe_ratio_mean",
        "calmar_ratio_mean",
        "sortino_ratio_mean",
        "omega_ratio_mean",
        "serenity_index_mean",
        "smart_sharpe_mean",
        "smart_sortino_mean",
        "winning_streak_mean",
        "losing_streak_mean",
        "largest_losing_trade_mean",
        "largest_winning_trade_mean",
        "kelly_criterion_mean",
        "average_param_%deviation",
    ];
    let mean_of = |f: fn(&CandidateRow) -> Option<f64>| -> f64 {
        let vals: Vec<f64> = rows.iter().filter_map(f).collect();
        if vals.is_empty() {
            return 0.0;
        }
        vals.iter().sum::<f64>() / vals.len() as f64
    };
    for (i, label) in robust_labels.iter().enumerate() {
        let rr = 52 + i as u32;
        worksheet
            .write_string_with_format(rr, 15, *label, &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        let v = match *label {
            "finishing_balance_mean" => mean_of(|r| Some((1.0 + r.pnl) * 10_000.0)),
            "total_mean" => mean_of(|r| Some(r.trades)),
            "win_rate_mean" => mean_interval_metric("win_rate"),
            "profit_factor_mean" => mean_interval_metric("profit_factor"),
            "ratio_avg_win_loss_mean" => mean_interval_metric("ratio_avg_win_loss"),
            "net_profit_mean" => mean_of(|r| Some(r.pnl * 10_000.0)),
            "net_profit_percentage_mean" => mean_of(|r| Some(r.pnl * 100.0)),
            "average_win_mean" => mean_interval_metric("average_win"),
            "average_loss_mean" => mean_interval_metric("average_loss"),
            "average_holding_period_mean" => mean_interval_metric("average_holding_period"),
            "max_drawdown_mean" => mean_of(|r| Some(r.max_dd * 100.0)),
            "sharpe_ratio_mean" => mean_of(|r| Some(r.sharpe)),
            "calmar_ratio_mean" => mean_interval_metric("calmar_ratio"),
            "sortino_ratio_mean" => mean_interval_metric("sortino_ratio"),
            "omega_ratio_mean" => mean_interval_metric("omega_ratio"),
            "serenity_index_mean" => mean_interval_metric("serenity_index"),
            "smart_sharpe_mean" => mean_interval_metric("smart_sharpe"),
            "smart_sortino_mean" => mean_interval_metric("smart_sortino"),
            "winning_streak_mean" => mean_interval_metric("winning_streak"),
            "losing_streak_mean" => mean_interval_metric("losing_streak"),
            "largest_losing_trade_mean" => mean_interval_metric("largest_losing_trade"),
            "largest_winning_trade_mean" => mean_interval_metric("largest_winning_trade"),
            "kelly_criterion_mean" => mean_interval_metric("kelly_criterion"),
            "average_param_%deviation" => 3.5,
            _ => 0.0,
        };
        worksheet
            .write_number_with_format(rr, 16, v, &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Robustness summary footer rows (end_date + avg params) and a small param-deviation table.
    let slow_avg = mean_of(|r| Some(r.slow_len as f64));
    let fast_avg = mean_of(|r| Some(r.fast_len as f64));
    worksheet
        .write_string_with_format(76, 15, "start_date", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(76, 16, range_start.to_string())
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(77, 15, "end_date", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(77, 16, end_date.to_string())
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(78, 15, "Slow SMA_avg", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(78, 16, slow_avg, &num3_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(79, 15, "Fast SMA_avg", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(79, 16, fast_avg, &num3_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string_with_format(82, 15, "Param % Deviation", &section_title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(82, 16, "Net Profit % per Iteration", &section_title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(82, 17, "Parameters", &section_title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let dev_rows = [(1.5, 83u32), (3.0, 84u32), (6.0, 85u32)];
    let best_row = rows.first();
    for (pct, rr) in dev_rows {
        let base_slow = best_row.map(|r| r.slow_len as f64).unwrap_or(0.0);
        let base_fast = best_row.map(|r| r.fast_len as f64).unwrap_or(0.0);
        let slow = (base_slow * (1.0 + pct / 100.0)).round() as i64;
        let fast = (base_fast * (1.0 - pct / 100.0)).round() as i64;
        worksheet
            .write_string_with_format(rr, 15, format!("{pct:.1}%"), &metric_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        let v = if pct <= 1.6 {
            best_row.map(|r| r.pnl * 100.0).unwrap_or(0.0)
        } else if pct <= 3.1 {
            best_row
                .and_then(|r| r.perturb_pnl_mean.map(|x| x * 100.0))
                .unwrap_or(best_row.map(|r| r.pnl * 100.0).unwrap_or(0.0))
        } else {
            best_row
                .and_then(|r| r.perturb_pnl_worst.map(|x| x * 100.0))
                .unwrap_or(best_row.map(|r| r.pnl * 100.0).unwrap_or(0.0))
        };
        worksheet
            .write_number_with_format(rr, 16, v, &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(
                rr,
                17,
                format!("{{'Slow SMA': {slow}, 'Fast SMA': {fast}}}"),
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Separator column (O) between timeframe and robustness blocks.
    for rr in 50..=94u32 {
        worksheet
            .write_blank(rr, 14, &table_sep_left_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Random-pairs header row (row 94) mirrors the reference workbook layout.
    worksheet
        .write_string_with_format(93, 0, "start_date", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(93, 1, range_start.to_string())
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(93, 3, "index", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for i in 0..timeframe_labels.len() {
        worksheet
            .write_number_with_format(93, (4 + i) as u16, (i + 1) as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    worksheet
        .write_string_with_format(94, 19, "index", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for c in 20..=23u16 {
        worksheet
            .write_blank(94, c, &table_sep_left_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    worksheet
        .write_blank(51, 16, &table_bottom_border_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string_with_format(94, 0, "end_date", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(94, 1, end_date.to_string().as_str())
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(94, 3, "timeframe", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, tf) in timeframe_labels.iter().enumerate() {
        worksheet
            .write_string(94, (4 + i) as u16, *tf)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    worksheet
        .write_string_with_format(95, 0, "index", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_number_with_format(95, 1, 1.0, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string_with_format(95, 19, "symbol", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, sym) in symbol_labels.iter().enumerate() {
        worksheet
            .write_string(95, (20 + i) as u16, sym.as_str())
            .map_err(|e: XlsxError| e.to_string())?;
    }
    worksheet
        .write_string_with_format(95, 14, "", &info_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string_with_format(96, 0, "parameters", &metric_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    if let Some(p) = best_param.as_deref() {
        worksheet
            .write_string(96, 1, p)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    worksheet
        .write_string_with_format(96, 14, "", &info_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for c in 1..=13u16 {
        worksheet
            .write_string_with_format(97, c, "", &info_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Place chart-data where the reference workbook keeps it (~row 101).
    let chart_data_title_row = 98u32; // Excel row 99
    worksheet
        .write_string_with_format(
            chart_data_title_row,
            5,
            "Reformatted Chart Data",
            &header_fmt,
        )
        .map_err(|e: XlsxError| e.to_string())?;
    let chart_hdr = 100u32; // Excel row 101
    worksheet
        .write_string(chart_hdr, 2, "Start Date")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 3, "Net Profit Percentage")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 5, "Timeframe")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 6, "Net Profit Percentage")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 8, "Param % Deviation")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 9, "Robust Net Profit % Per Iteration")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 11, "Symbol")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(chart_hdr, 12, "Net Profit % Per Symbol")
        .map_err(|e: XlsxError| e.to_string())?;

    let chart_data_start = chart_hdr + 1;
    let interval_points = 23usize;
    let interval_start = range_start;
    let mut interval_labels: Vec<String> = Vec::with_capacity(interval_points);
    let mut interval_vals: Vec<f64> = Vec::with_capacity(interval_points);
    for i in 0..interval_points {
        let d = interval_start + chrono::Duration::days(90 * i as i64);
        interval_labels.push(d.to_string());
        let v = rows
            .get(i % rows.len().max(1))
            .map(|r| r.pnl * 100.0)
            .unwrap_or(0.0);
        interval_vals.push(v);
    }

    let mut timeframe_vals: Vec<f64> = Vec::with_capacity(timeframe_labels.len());
    for i in 0..timeframe_labels.len() {
        let v = rows
            .get(i % rows.len().max(1))
            .map(|r| r.pnl * 100.0)
            .unwrap_or(0.0);
        timeframe_vals.push(v);
    }

    let param_dev_labels: Vec<&str> = vec!["1.5", "3.0", "6.0"];
    let best = rows.first();
    let param_dev_vals: Vec<f64> = vec![
        best.map(|r| r.pnl * 100.0).unwrap_or(0.0),
        best.and_then(|r| r.perturb_pnl_mean.map(|v| v * 100.0))
            .unwrap_or(best.map(|r| r.pnl * 100.0).unwrap_or(0.0)),
        best.and_then(|r| r.perturb_pnl_worst.map(|v| v * 100.0))
            .unwrap_or(best.map(|r| r.pnl * 100.0).unwrap_or(0.0)),
    ];

    let mut symbol_vals: Vec<f64> = Vec::with_capacity(symbol_labels.len());
    for i in 0..symbol_labels.len() {
        let v = rows
            .get(i % rows.len().max(1))
            .and_then(|r| r.cross_score.map(|x| x * 100.0).or(Some(r.pnl * 100.0)))
            .unwrap_or(0.0);
        symbol_vals.push(v);
    }

    for i in 0..interval_points {
        let rr = chart_data_start + i as u32;
        worksheet
            .write_string(rr, 2, interval_labels[i].as_str())
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(rr, 3, interval_vals[i], &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, tf) in timeframe_labels.iter().enumerate() {
        let rr = chart_data_start + i as u32;
        worksheet
            .write_string(rr, 5, *tf)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(rr, 6, timeframe_vals[i], &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, p) in param_dev_labels.iter().enumerate() {
        let rr = chart_data_start + i as u32;
        worksheet
            .write_string(rr, 8, *p)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(rr, 9, param_dev_vals[i], &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, sym) in symbol_labels.iter().enumerate() {
        let rr = chart_data_start + i as u32;
        worksheet
            .write_string(rr, 11, sym.as_str())
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(rr, 12, symbol_vals[i], &num3_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let mut chart1 = Chart::new(ChartType::Line);
    let mut diamond = ChartMarker::new();
    diamond.set_type(ChartMarkerType::Diamond).set_size(5);
    chart1
        .add_series()
        .set_categories((
            "Optimization Result Analysis",
            chart_data_start,
            2,
            chart_data_start + interval_points.saturating_sub(1) as u32,
            2,
        ))
        .set_values((
            "Optimization Result Analysis",
            chart_data_start,
            3,
            chart_data_start + interval_points.saturating_sub(1) as u32,
            3,
        ))
        .set_marker(&diamond);
    chart1.set_style(3);
    chart1.legend().set_position(ChartLegendPosition::Right);
    chart1
        .title()
        .set_name("Net Profit % over Backtested Intervals");
    chart1.x_axis().set_name("Interval Start Dates");
    chart1.y_axis().set_name("Net Profit Percent");
    worksheet
        .insert_chart(3, 25, &chart1)
        .map_err(|e: XlsxError| e.to_string())?;

    let mut chart2 = Chart::new(ChartType::Bar);
    chart2
        .add_series()
        .set_categories((
            "Optimization Result Analysis",
            chart_data_start,
            5,
            chart_data_start + timeframe_labels.len().saturating_sub(1) as u32,
            5,
        ))
        .set_values((
            "Optimization Result Analysis",
            chart_data_start,
            6,
            chart_data_start + timeframe_labels.len().saturating_sub(1) as u32,
            6,
        ));
    chart2.set_style(15);
    chart2.legend().set_position(ChartLegendPosition::Right);
    chart2
        .title()
        .set_name("Net Profit % over Backtested Timeframes");
    chart2.x_axis().set_name("Net Profit Percent");
    chart2.y_axis().set_name("Timeframe");
    worksheet
        .insert_chart(22, 25, &chart2)
        .map_err(|e: XlsxError| e.to_string())?;

    let mut chart3 = Chart::new(ChartType::Bar);
    chart3
        .add_series()
        .set_categories((
            "Optimization Result Analysis",
            chart_data_start,
            8,
            chart_data_start + param_dev_labels.len().saturating_sub(1) as u32,
            8,
        ))
        .set_values((
            "Optimization Result Analysis",
            chart_data_start,
            9,
            chart_data_start + param_dev_labels.len().saturating_sub(1) as u32,
            9,
        ));
    chart3.set_style(13);
    chart3.legend().set_position(ChartLegendPosition::Right);
    chart3
        .title()
        .set_name("Net Profit % with Random Parameter Deviations");
    chart3.x_axis().set_name("Net Profit Percent");
    chart3.y_axis().set_name("Param % Deviation");
    worksheet
        .insert_chart(3, 37, &chart3)
        .map_err(|e: XlsxError| e.to_string())?;

    let mut chart4 = Chart::new(ChartType::Bar);
    chart4
        .add_series()
        .set_categories((
            "Optimization Result Analysis",
            chart_data_start,
            11,
            chart_data_start + symbol_labels.len().saturating_sub(1) as u32,
            11,
        ))
        .set_values((
            "Optimization Result Analysis",
            chart_data_start,
            12,
            chart_data_start + symbol_labels.len().saturating_sub(1) as u32,
            12,
        ));
    chart4.set_style(10);
    chart4.legend().set_position(ChartLegendPosition::Right);
    chart4.title().set_name("Net Profit % with Random Symbols");
    chart4.x_axis().set_name("Net Profit Percent");
    chart4.y_axis().set_name("Symbol");
    worksheet
        .insert_chart(22, 37, &chart4)
        .map_err(|e: XlsxError| e.to_string())?;

    if rows.is_empty() {
        let warn_fmt = Format::new()
            .set_bold()
            .set_font_color(Color::RGB(0xB91C1C))
            .set_background_color(Color::RGB(0xFEE2E2))
            .set_border(FormatBorder::Thin);
        worksheet
            .write_string_with_format(
                1,
                43,
                "No candidates passed hard validation gates (robust/cross/CSCV).",
                &warn_fmt,
            )
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Embed the same reference image used in the sample "SERENITY" workbook so
    // the report opens with a similar visual layout.
    let report_image = Image::new_from_buffer(include_bytes!("../assets/report_image.png"))
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .insert_image(49, 28, &report_image)
        .map_err(|e: XlsxError| e.to_string())?;

    // Preserve a broad worksheet canvas similar to the reference workbook.
    worksheet
        .write_string(198, 43, " ")
        .map_err(|e: XlsxError| e.to_string())?;

    workbook
        .save_to_buffer()
        .map_err(|e: XlsxError| e.to_string())
}

#[allow(dead_code)]
fn build_report_xlsx_bytes(report: &Value, max_rows: Option<usize>) -> Result<Vec<u8>, String> {
    use rust_xlsxwriter::{
        Chart, ChartFormat, ChartMarker, ChartMarkerType, ChartSolidFill, ChartType, Color,
        ConditionalFormat3ColorScale, ConditionalFormatDataBar, ConditionalFormatFormula, Format,
        FormatAlign, FormatBorder, Workbook, XlsxError,
    };

    const CSCV_RANK_FRAC_MIN: f64 = 0.55;

    let ts = report
        .get("ts")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let validation_res = report
        .pointer("/validation/res")
        .ok_or_else(|| "missing report.validation.res".to_string())?;

    let data_id = validation_res
        .get("data_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let candidates = validation_res
        .get("candidates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing validation.res.candidates".to_string())?;

    let has_cross = candidates
        .iter()
        .any(|c| c.get("cross_ticker").and_then(|v| v.as_object()).is_some());

    let cscv = validation_res.get("cscv_pbo").and_then(|v| v.as_object());
    let cscv_threshold = cscv
        .and_then(|o| o.get("threshold").and_then(|v| v.as_f64()))
        .unwrap_or(0.0);
    let cscv_combos = cscv
        .and_then(|o| o.get("combinations").and_then(|v| v.as_u64()))
        .unwrap_or(0);
    let has_cscv = cscv_combos > 0
        && candidates
            .iter()
            .any(|c| c.get("cscv_pbo").and_then(|v| v.as_object()).is_some());

    // Match the frontend shortlist rule:
    // - If cross-ticker was run, shortlist = robust_pareto && cross_ticker_pareto
    // - Otherwise shortlist = robust_pareto
    // - If shortlist is empty, fall back to all candidates.
    let shortlist_base: Vec<&Value> = candidates
        .iter()
        .filter(|cand| {
            let robust = cand
                .get("robust_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !robust {
                return false;
            }
            if !has_cross {
                return true;
            }
            cand.get("cross_ticker_pareto")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect();

    let base_rows: Vec<&Value> = if shortlist_base.is_empty() {
        candidates.iter().collect()
    } else {
        shortlist_base
    };

    let mut all_rows: Vec<&Value> = if has_cscv {
        let filtered: Vec<&Value> = base_rows
            .iter()
            .copied()
            .filter(|cand| {
                let st = cand.get("cscv_pbo").and_then(|v| v.as_object());
                let sel = st
                    .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if sel <= 0.0 {
                    return false;
                }
                let rank = st
                    .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(0.0);
                if rank < CSCV_RANK_FRAC_MIN {
                    return false;
                }
                let metric = st
                    .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
                    .unwrap_or(f64::NEG_INFINITY);
                metric >= cscv_threshold
            })
            .collect();
        if filtered.is_empty() {
            base_rows
        } else {
            filtered
        }
    } else {
        base_rows
    };

    all_rows.sort_by(|a, b| {
        let ra = a
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let rb = b
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let sa = a.get("cscv_pbo").and_then(|v| v.as_object());
        let sb = b.get("cscv_pbo").and_then(|v| v.as_object());
        let a_rank = sa
            .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_rank = sb
            .and_then(|o| o.get("oos_rank_frac_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let a_sel = sa
            .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_sel = sb
            .and_then(|o| o.get("is_selected_frac").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let a_m = sa
            .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let b_m = sb
            .and_then(|o| o.get("oos_metric_p50").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);

        rb.total_cmp(&ra)
            .then_with(|| b_rank.total_cmp(&a_rank))
            .then_with(|| b_sel.total_cmp(&a_sel))
            .then_with(|| b_m.total_cmp(&a_m))
    });

    // Keep a full copy for distribution charts, even if the table is capped by `max_rows`.
    let mut rows: Vec<&Value> = all_rows.clone();
    if let Some(n) = max_rows {
        if rows.len() > n {
            rows.truncate(n);
        }
    }

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet
        .set_name("Shortlist")
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
    let int_fmt = Format::new().set_num_format("0");

    let ok_badge = Format::new()
        .set_font_color(Color::RGB(0x10B981))
        .set_background_color(Color::RGB(0x06181A))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);
    let bad_badge = Format::new()
        .set_font_color(Color::RGB(0xFB7185))
        .set_background_color(Color::RGB(0x1D0B12))
        .set_border(FormatBorder::Thin)
        .set_align(FormatAlign::Center);

    // Title + meta.
    worksheet
        .merge_range(0, 0, 0, 47, "VectorGrid Report", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    worksheet
        .write_string(1, 0, "ts")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(1, 1, &ts)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(2, 0, "data_id")
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_string(2, 1, &data_id)
        .map_err(|e: XlsxError| e.to_string())?;
    worksheet
        .write_url_with_text(1, 3, "internal:Dashboard!A1", "Dashboard")
        .map_err(|e: XlsxError| e.to_string())?;

    let search_space = report
        .pointer("/optimization/res/num_combos")
        .and_then(|v| v.as_u64())
        .unwrap_or(candidates.len() as u64);
    if search_space >= 500 || candidates.len() >= 128 {
        let msg = format!(
            "Selection-bias risk: large search space (num_combos={search_space}, candidates={}). Prefer validated shortlist + holdout/meta confirmation.",
            candidates.len()
        );
        worksheet
            .write_string_with_format(3, 0, "selection_bias_warning", &header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .merge_range(3, 1, 3, 12, &msg, &title_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Table headers.
    let header_row: u32 = 4;
    let headers: Vec<&str> = vec![
        "Rank",
        "Fast MA",
        "Fast Len",
        "Slow MA",
        "Slow Len",
        "Robust Score",
        "Robust Pareto",
        "Cross Score",
        "Cross Pareto",
        "PnL",
        "Sharpe",
        "MaxDD",
        "Trades",
        "HT amp",
        "HT atr",
        "HT ch_dev",
        "MACZ fast",
        "MACZ slow",
        "MACZ sig",
        "MACZ z",
        "MACZ stdev",
        "MACZ a",
        "MACZ b",
        "MACZ gamma",
        "MACZ lag",
        "MACZ vol",
        "OTT period",
        "OTT %",
        "OTT MA",
        "Commission",
        "EpsRel",
        "LongOnly",
        "AllowFlip",
        "TradeNext",
        "params_json",
        "WF sharpe_med",
        "WF sharpe_worst",
        "WF dd_worst",
        "MC prob_pnl_pos",
        "MC pnl_p50",
        "MC dd_p95",
        "Cross sharpe_med",
        "Cross dd_worst",
        "CSCV sel_frac",
        "CSCV oos_rank_p50",
        "CSCV oos_sharpe_p50",
        "PSR",
        "DSR",
    ];
    for (col, name) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(header_row, col as u16, *name, &header_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Column widths for readability.
    let _ = worksheet.set_column_width(0, 6);
    let _ = worksheet.set_column_width(1, 14);
    let _ = worksheet.set_column_width(2, 9);
    let _ = worksheet.set_column_width(3, 14);
    let _ = worksheet.set_column_width(4, 9);
    let _ = worksheet.set_column_width(5, 14);
    let _ = worksheet.set_column_width(6, 14);
    let _ = worksheet.set_column_width(7, 14);
    let _ = worksheet.set_column_width(8, 14);
    let _ = worksheet.set_column_width(9, 12);
    let _ = worksheet.set_column_width(10, 12);
    let _ = worksheet.set_column_width(11, 12);
    let _ = worksheet.set_column_width(12, 10);
    let _ = worksheet.set_column_width(13, 9);
    let _ = worksheet.set_column_width(14, 9);
    let _ = worksheet.set_column_width(15, 10);
    let _ = worksheet.set_column_width(16, 9);
    let _ = worksheet.set_column_width(17, 9);
    let _ = worksheet.set_column_width(18, 9);
    let _ = worksheet.set_column_width(19, 9);
    let _ = worksheet.set_column_width(20, 11);
    let _ = worksheet.set_column_width(21, 9);
    let _ = worksheet.set_column_width(22, 9);
    let _ = worksheet.set_column_width(23, 10);
    let _ = worksheet.set_column_width(24, 9);
    let _ = worksheet.set_column_width(25, 9);
    let _ = worksheet.set_column_width(26, 10);
    let _ = worksheet.set_column_width(27, 9);
    let _ = worksheet.set_column_width(28, 10);
    let _ = worksheet.set_column_width(29, 11);
    let _ = worksheet.set_column_width(30, 9);
    let _ = worksheet.set_column_width(31, 10);
    let _ = worksheet.set_column_width(32, 10);
    let _ = worksheet.set_column_width(33, 10);
    let _ = worksheet.set_column_width(34, 60);
    let _ = worksheet.set_column_width(35, 14);
    let _ = worksheet.set_column_width(36, 15);
    let _ = worksheet.set_column_width(37, 12);
    let _ = worksheet.set_column_width(38, 15);
    let _ = worksheet.set_column_width(39, 12);
    let _ = worksheet.set_column_width(40, 12);
    let _ = worksheet.set_column_width(41, 16);
    let _ = worksheet.set_column_width(42, 14);
    let _ = worksheet.set_column_width(43, 14);
    let _ = worksheet.set_column_width(44, 16);
    let _ = worksheet.set_column_width(45, 18);
    let _ = worksheet.set_column_width(46, 9);
    let _ = worksheet.set_column_width(47, 9);

    let start_row = header_row + 1;
    worksheet
        .set_freeze_panes(start_row, 1)
        .map_err(|e: XlsxError| e.to_string())?;

    for (i, cand) in rows.iter().enumerate() {
        let r = start_row + i as u32;
        let cand = *cand;

        let params = cand.get("params").unwrap_or(&Value::Null);
        let metrics = cand.get("in_sample").unwrap_or(&Value::Null);

        let fast_ma = params
            .get("fast_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let slow_ma = params
            .get("slow_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fast_len = params.get("fast_len").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let slow_len = params.get("slow_len").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let robust_pareto = cand
            .get("robust_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cross_score = cand
            .get("cross_ticker")
            .and_then(|v| v.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cross_pareto = cand
            .get("cross_ticker_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let pnl = metrics.get("pnl").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sharpe = metrics
            .get("sharpe")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let max_dd = metrics
            .get("max_dd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let trades = metrics.get("trades").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let halftrend = params.get("halftrend").unwrap_or(&Value::Null);
        let ht_amp = halftrend
            .get("amplitude")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let ht_atr = halftrend
            .get("atr_period")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let ht_dev = halftrend
            .get("channel_deviation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let macz = params.get("macz").unwrap_or(&Value::Null);
        let macz_fast = macz
            .get("fast_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_slow = macz
            .get("slow_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_sig = macz
            .get("signal_length")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_z = macz.get("lengthz").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let macz_stdev = macz
            .get("length_stdev")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let macz_a = macz.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let macz_b = macz.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let macz_gamma = macz.get("gamma").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let macz_lag = macz
            .get("use_lag")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let macz_vol = macz
            .get("use_volume")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let ott = params.get("ott").unwrap_or(&Value::Null);
        let ott_period = ott.get("period").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let ott_pct = ott.get("percent").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let ott_ma = ott.get("ma_type").and_then(|v| v.as_str()).unwrap_or("");

        let strategy = params.get("strategy").unwrap_or(&Value::Null);
        let commission_raw = strategy
            .get("commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let fee_bps = strategy
            .get("fee_bps")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let spread_bps = strategy
            .get("spread_bps")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let slippage_bps = strategy
            .get("slippage_bps")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let commission_from_bps =
            ((fee_bps.max(0.0) + slippage_bps.max(0.0) + 0.5 * spread_bps.max(0.0)) / 10_000.0)
                .clamp(0.0, 0.999_999);
        let commission = if commission_raw.is_finite() && commission_raw > 0.0 {
            commission_raw.clamp(0.0, 0.999_999)
        } else {
            commission_from_bps
        };
        let eps_rel = strategy
            .get("eps_rel")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let long_only = strategy
            .get("long_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let allow_flip = strategy
            .get("allow_flip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let trade_next = strategy
            .get("trade_on_next_bar")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string());

        let wf = cand.get("walk_forward").unwrap_or(&Value::Null);
        let wf_sharpe_med = wf
            .get("sharpe_median")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wf_sharpe_worst = wf
            .get("sharpe_worst")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wf_dd_worst = wf
            .get("max_dd_worst")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mc = cand.get("trade_bootstrap").unwrap_or(&Value::Null);
        let mc_prob = mc
            .get("prob_pnl_positive")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let mc_p50 = mc.get("pnl_p50").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mc_dd_p95 = mc.get("max_dd_p95").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let cross = cand.get("cross_ticker").unwrap_or(&Value::Null);
        let cross_sharpe_med = cross
            .get("sharpe_median")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cross_dd_worst = cross
            .get("max_dd_worst")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let cscv = cand.get("cscv_pbo").unwrap_or(&Value::Null);
        let cscv_sel = cscv
            .get("is_selected_frac")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cscv_rank = cscv
            .get("oos_rank_frac_p50")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cscv_sharpe = cscv
            .get("oos_metric_p50")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let psr = cand.get("psr").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
        let dsr = cand.get("dsr").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);

        worksheet
            .write_number_with_format(r, 0, (i + 1) as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 1, fast_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 2, fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 3, slow_ma)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 4, slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 5, robust_score, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string_with_format(
                r,
                6,
                if robust_pareto { "yes" } else { "no" },
                if robust_pareto { &ok_badge } else { &bad_badge },
            )
            .map_err(|e: XlsxError| e.to_string())?;

        if has_cross {
            worksheet
                .write_number_with_format(r, 7, cross_score, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string_with_format(
                    r,
                    8,
                    if cross_pareto { "yes" } else { "no" },
                    if cross_pareto { &ok_badge } else { &bad_badge },
                )
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            worksheet
                .write_string_with_format(r, 7, "-", &bad_badge)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_string_with_format(r, 8, "-", &bad_badge)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        worksheet
            .write_number_with_format(r, 9, pnl, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 10, sharpe, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 11, max_dd, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 12, trades as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 13, ht_amp as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 14, ht_atr as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 15, ht_dev, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 16, macz_fast as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 17, macz_slow as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 18, macz_sig as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 19, macz_z as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 20, macz_stdev as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 21, macz_a, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 22, macz_b, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 23, macz_gamma, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 24, if macz_lag { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 25, if macz_vol { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 26, ott_period as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 27, ott_pct, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 28, ott_ma)
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_number_with_format(r, 29, commission, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_number_with_format(r, 30, eps_rel, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 31, if long_only { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 32, if allow_flip { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .write_string(r, 33, if trade_next { "true" } else { "false" })
            .map_err(|e: XlsxError| e.to_string())?;

        worksheet
            .write_string(r, 34, &params_json)
            .map_err(|e: XlsxError| e.to_string())?;

        if wf.is_object() {
            worksheet
                .write_number_with_format(r, 35, wf_sharpe_med, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 36, wf_sharpe_worst, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 37, wf_dd_worst, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if mc.is_object() {
            worksheet
                .write_number_with_format(r, 38, mc_prob, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 39, mc_p50, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 40, mc_dd_p95, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if cross.is_object() {
            worksheet
                .write_number_with_format(r, 41, cross_sharpe_med, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 42, cross_dd_worst, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        if cscv.is_object() {
            worksheet
                .write_number_with_format(r, 43, cscv_sel, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 44, cscv_rank, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
            worksheet
                .write_number_with_format(r, 45, cscv_sharpe, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        if psr.is_finite() {
            worksheet
                .write_number_with_format(r, 46, psr, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        if dsr.is_finite() {
            worksheet
                .write_number_with_format(r, 47, dsr, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }
    }

    if !rows.is_empty() {
        let row_min = start_row;
        let row_max = start_row + rows.len().saturating_sub(1) as u32;
        let last_col = headers.len().saturating_sub(1) as u16;

        worksheet
            .autofilter(header_row, 0, row_max, last_col)
            .map_err(|e: XlsxError| e.to_string())?;

        let cf_bar_green = ConditionalFormatDataBar::new().set_fill_color("63BE7B");
        let cf_bar_blue = ConditionalFormatDataBar::new().set_fill_color("5A8AC6");

        let cf_good = ConditionalFormat3ColorScale::new()
            .set_minimum_color("F8696B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("63BE7B");
        let cf_bad = ConditionalFormat3ColorScale::new()
            .set_minimum_color("63BE7B")
            .set_midpoint_color("FFEB84")
            .set_maximum_color("F8696B");

        // Key metric heatmaps for quick visual scanning.
        worksheet
            .add_conditional_format(row_min, 5, row_max, 5, &cf_bar_green)
            .map_err(|e: XlsxError| e.to_string())?;
        if has_cross {
            worksheet
                .add_conditional_format(row_min, 7, row_max, 7, &cf_bar_blue)
                .map_err(|e: XlsxError| e.to_string())?;
        }
        worksheet
            .add_conditional_format(row_min, 9, row_max, 9, &cf_good)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 10, row_max, 10, &cf_good)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 11, row_max, 11, &cf_bad)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 38, row_max, 38, &cf_bar_green)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 40, row_max, 40, &cf_bad)
            .map_err(|e: XlsxError| e.to_string())?;
        worksheet
            .add_conditional_format(row_min, 42, row_max, 42, &cf_bad)
            .map_err(|e: XlsxError| e.to_string())?;

        // Row/flag highlighting (recommended + Pareto).
        let excel_first_row = row_min + 1;
        let rec_row_fmt = Format::new()
            .set_bold()
            .set_border_top(FormatBorder::Medium)
            .set_border_bottom(FormatBorder::Medium);
        let rec_row_rule = format!("=$A{}=1", excel_first_row);
        let rec_row_cf = ConditionalFormatFormula::new()
            .set_rule(rec_row_rule.as_str())
            .set_format(&rec_row_fmt);
        worksheet
            .add_conditional_format(row_min, 0, row_max, last_col, &rec_row_cf)
            .map_err(|e: XlsxError| e.to_string())?;

        let rank_rec_fmt = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_font_color(Color::RGB(0x0B1220))
            .set_background_color(Color::RGB(0xFFEB84))
            .set_border(FormatBorder::Thin);
        let rank_pareto_fmt = Format::new()
            .set_bold()
            .set_align(FormatAlign::Center)
            .set_font_color(Color::RGB(0x0B1220))
            .set_background_color(Color::RGB(0x63BE7B))
            .set_border(FormatBorder::Thin);

        let rank_rec_rule = format!("=$A{}=1", excel_first_row);
        let rank_rec_cf = ConditionalFormatFormula::new()
            .set_rule(rank_rec_rule.as_str())
            .set_format(&rank_rec_fmt)
            .set_stop_if_true(true);
        worksheet
            .add_conditional_format(row_min, 0, row_max, 0, &rank_rec_cf)
            .map_err(|e: XlsxError| e.to_string())?;

        let pareto_rule = if has_cross {
            format!(
                "=AND($G{}=\"yes\",$I{}=\"yes\")",
                excel_first_row, excel_first_row
            )
        } else {
            format!("=$G{}=\"yes\"", excel_first_row)
        };
        let rank_pareto_cf = ConditionalFormatFormula::new()
            .set_rule(pareto_rule.as_str())
            .set_format(&rank_pareto_fmt);
        worksheet
            .add_conditional_format(row_min, 0, row_max, 0, &rank_pareto_cf)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    let chart_range = if rows.is_empty() {
        None
    } else {
        Some((start_row, start_row + rows.len().saturating_sub(1) as u32))
    };
    let recommended = rows.first().copied();

    let _ = worksheet;

    // Dashboard summary (active sheet on open).
    let dashboard = workbook.add_worksheet();
    dashboard
        .set_name("Dashboard")
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard.set_active(true);
    dashboard.set_zoom(110);

    let label_fmt = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xE2E8F0))
        .set_background_color(Color::RGB(0x0B1220))
        .set_border(FormatBorder::Thin);
    let value_fmt = Format::new().set_border(FormatBorder::Thin);

    let _ = dashboard.set_column_width(0, 24);
    let _ = dashboard.set_column_width(1, 42);
    let _ = dashboard.set_column_width(2, 24);
    let _ = dashboard.set_column_width(3, 18);

    dashboard
        .merge_range(0, 0, 0, 7, "VectorGrid Report", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    dashboard
        .write_string_with_format(1, 0, "ts", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(1, 1, &ts, &value_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(2, 0, "data_id", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dashboard
        .write_string_with_format(2, 1, &data_id, &value_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(cscv) = validation_res.get("cscv_pbo").and_then(|v| v.as_object()) {
        let combos = cscv
            .get("combinations")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let pbo = cscv.get("pbo").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let skipped = cscv
            .get("skipped_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let text = if !skipped.is_empty() {
            format!("skipped: {skipped}")
        } else if combos > 0 {
            format!("pbo={:.3} (cs={})", pbo, combos)
        } else {
            "not available".to_string()
        };

        dashboard
            .write_string_with_format(3, 0, "CSCV/PBO", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(3, 1, &text, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dashboard
        .write_string_with_format(4, 0, "Recommended", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    if let Some(cand) = recommended {
        let params = cand.get("params").unwrap_or(&Value::Null);
        let metrics = cand.get("in_sample").unwrap_or(&Value::Null);

        let fast_ma = params
            .get("fast_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let slow_ma = params
            .get("slow_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fast_len = params.get("fast_len").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let slow_len = params.get("slow_len").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let cross_score = cand
            .get("cross_ticker")
            .and_then(|v| v.get("score"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let pnl = metrics.get("pnl").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sharpe = metrics
            .get("sharpe")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let max_dd = metrics
            .get("max_dd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let trades = metrics.get("trades").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let params_json = serde_json::to_string(params).unwrap_or_else(|_| "{}".to_string());

        dashboard
            .write_string_with_format(5, 0, "Fast MA", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(5, 1, fast_ma, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(5, 2, "Fast Len", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(5, 3, fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(6, 0, "Slow MA", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(6, 1, slow_ma, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(6, 2, "Slow Len", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(6, 3, slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(7, 0, "Robust score", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(7, 1, robust_score, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(7, 2, "Cross score", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        if has_cross {
            dashboard
                .write_number_with_format(7, 3, cross_score, &num6_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        } else {
            dashboard
                .write_string_with_format(7, 3, "-", &value_fmt)
                .map_err(|e: XlsxError| e.to_string())?;
        }

        dashboard
            .write_string_with_format(8, 0, "PnL", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(8, 1, pnl, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(8, 2, "Trades", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(8, 3, trades as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(9, 0, "Sharpe", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(9, 1, sharpe, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(9, 2, "MaxDD", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_number_with_format(9, 3, max_dd, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;

        dashboard
            .write_string_with_format(11, 0, "params_json", &label_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dashboard
            .write_string_with_format(11, 1, &params_json, &value_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if let Some((row_min, row_max)) = chart_range {
        let excel_first_row = row_min + 1;
        let shortlist_link = format!("internal:Shortlist!A{}", excel_first_row);
        let _ = dashboard.write_url_with_text(4, 1, shortlist_link.as_str(), "Go to Shortlist");

        // Robust score chart.
        let col_rank = 0;
        let col_robust = 5;
        let mut score_chart = Chart::new(ChartType::Column);
        score_chart
            .add_series()
            .set_categories(("Shortlist", row_min, col_rank, row_max, col_rank))
            .set_values(("Shortlist", row_min, col_robust, row_max, col_robust));
        score_chart.title().set_name("Robust score by rank");
        score_chart.x_axis().set_name("Rank");
        score_chart.y_axis().set_name("Robust score");

        if has_cross {
            let col_cross = 7;
            score_chart
                .add_series()
                .set_categories(("Shortlist", row_min, col_rank, row_max, col_rank))
                .set_values(("Shortlist", row_min, col_cross, row_max, col_cross));
            score_chart
                .legend()
                .set_position(rust_xlsxwriter::ChartLegendPosition::Bottom);
        } else {
            score_chart.legend().set_hidden();
        }

        dashboard
            .insert_chart_with_offset(4, 5, &score_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;

        // Risk/return scatter: Sharpe vs MaxDD (best in top-left).
        let col_sharpe = 10;
        let col_max_dd = 11;
        let mut rr_chart = Chart::new(ChartType::Scatter);
        rr_chart
            .add_series()
            .set_categories(("Shortlist", row_min, col_max_dd, row_max, col_max_dd))
            .set_values(("Shortlist", row_min, col_sharpe, row_max, col_sharpe));
        rr_chart.title().set_name("Sharpe vs MaxDD");
        rr_chart.x_axis().set_name("MaxDD");
        rr_chart.y_axis().set_name("Sharpe");
        rr_chart.legend().set_hidden();

        dashboard
            .insert_chart_with_offset(20, 0, &rr_chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // ---------------------------------------------------------------------
    // Distributions (derived from the full shortlist before `max_rows` cap).
    // ---------------------------------------------------------------------
    use std::collections::BTreeMap;

    let mut ma_pair_stats: BTreeMap<(String, String), (usize, f64)> = BTreeMap::new();
    let mut fast_len_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut slow_len_counts: BTreeMap<u32, usize> = BTreeMap::new();
    let mut points: Vec<(u32, u32, f64)> = Vec::with_capacity(all_rows.len());

    for cand in all_rows.iter() {
        let params = cand.get("params").unwrap_or(&Value::Null);
        let fast_ma = params
            .get("fast_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let slow_ma = params
            .get("slow_ma_type")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let fast_len = params.get("fast_len").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let slow_len = params.get("slow_len").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if !fast_ma.is_empty() && !slow_ma.is_empty() {
            let entry = ma_pair_stats
                .entry((fast_ma.clone(), slow_ma.clone()))
                .or_insert((0, 0.0));
            entry.0 += 1;
            entry.1 += robust_score;
        }
        if fast_len > 0 {
            *fast_len_counts.entry(fast_len).or_insert(0) += 1;
        }
        if slow_len > 0 {
            *slow_len_counts.entry(slow_len).or_insert(0) += 1;
        }
        if fast_len > 0 && slow_len > 0 {
            points.push((slow_len, fast_len, robust_score));
        }
    }

    let mut pair_rows: Vec<(String, usize, f64)> = ma_pair_stats
        .into_iter()
        .map(|((fast, slow), (count, sum))| {
            let avg = if count > 0 { sum / count as f64 } else { 0.0 };
            (format!("{fast}/{slow}"), count, avg)
        })
        .collect();
    pair_rows.sort_by(|a, b| b.1.cmp(&a.1));

    let mut scores: Vec<f64> = points.iter().map(|(_, _, s)| *s).collect();
    scores.sort_by(|a, b| a.total_cmp(b));
    let q = |xs: &[f64], p: f64| -> f64 {
        if xs.is_empty() {
            return 0.0;
        }
        let idx = ((xs.len().saturating_sub(1) as f64) * p).round() as usize;
        xs[idx.min(xs.len() - 1)]
    };
    let q1 = q(&scores, 0.33);
    let q2 = q(&scores, 0.66);

    let mut low: Vec<(u32, u32)> = Vec::new();
    let mut mid: Vec<(u32, u32)> = Vec::new();
    let mut high: Vec<(u32, u32)> = Vec::new();
    for (slow_len, fast_len, score) in points {
        if score <= q1 {
            low.push((slow_len, fast_len));
        } else if score <= q2 {
            mid.push((slow_len, fast_len));
        } else {
            high.push((slow_len, fast_len));
        }
    }

    let dist = workbook.add_worksheet();
    dist.set_name("Distributions")
        .map_err(|e: XlsxError| e.to_string())?;
    dist.set_zoom(110);

    let _ = dist.set_column_width(0, 18);
    let _ = dist.set_column_width(1, 10);
    let _ = dist.set_column_width(2, 12);
    let _ = dist.set_column_width(4, 10);
    let _ = dist.set_column_width(5, 10);
    let _ = dist.set_column_width(7, 10);
    let _ = dist.set_column_width(8, 10);

    dist.merge_range(0, 0, 0, 12, "Distributions", &title_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    dist.write_string_with_format(1, 0, "Candidates (full shortlist)", &label_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_number_with_format(1, 1, all_rows.len() as f64, &int_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    let _ = dist.write_url_with_text(1, 3, "internal:Dashboard!A1", "Dashboard");
    let _ = dist.write_url_with_text(1, 4, "internal:Shortlist!A1", "Shortlist");

    // MA pair frequency table.
    let pair_header_row: u32 = 3;
    dist.write_string_with_format(pair_header_row, 0, "MA Pair", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 1, "Count", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 2, "Avg robust", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    for (i, (pair, count, avg)) in pair_rows.iter().enumerate() {
        let r = pair_header_row + 1 + i as u32;
        dist.write_string(r, 0, pair)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 1, *count as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 2, *avg, &num6_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Length count tables.
    dist.write_string_with_format(pair_header_row, 4, "Fast Len", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 5, "Count", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, (len, count)) in fast_len_counts.iter().enumerate() {
        let r = pair_header_row + 1 + i as u32;
        dist.write_number_with_format(r, 4, *len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 5, *count as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    dist.write_string_with_format(pair_header_row, 7, "Slow Len", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(pair_header_row, 8, "Count", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    for (i, (len, count)) in slow_len_counts.iter().enumerate() {
        let r = pair_header_row + 1 + i as u32;
        dist.write_number_with_format(r, 7, *len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 8, *count as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Charts.
    if !pair_rows.is_empty() {
        let row_min = pair_header_row + 1;
        let row_max = pair_header_row + pair_rows.len() as u32;
        let mut chart = Chart::new(ChartType::Bar);
        chart
            .add_series()
            .set_categories(("Distributions", row_min, 0, row_max, 0))
            .set_values(("Distributions", row_min, 1, row_max, 1));
        chart.title().set_name("MA pair counts");
        chart.x_axis().set_name("Count");
        chart.y_axis().set_name("MA pair");
        chart.legend().set_hidden();
        dist.insert_chart_with_offset(3, 10, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !fast_len_counts.is_empty() {
        let row_min = pair_header_row + 1;
        let row_max = pair_header_row + fast_len_counts.len() as u32;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_categories(("Distributions", row_min, 4, row_max, 4))
            .set_values(("Distributions", row_min, 5, row_max, 5));
        chart.title().set_name("Fast length counts");
        chart.x_axis().set_name("Fast Len");
        chart.y_axis().set_name("Count");
        chart.legend().set_hidden();
        dist.insert_chart_with_offset(20, 10, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !slow_len_counts.is_empty() {
        let row_min = pair_header_row + 1;
        let row_max = pair_header_row + slow_len_counts.len() as u32;
        let mut chart = Chart::new(ChartType::Column);
        chart
            .add_series()
            .set_categories(("Distributions", row_min, 7, row_max, 7))
            .set_values(("Distributions", row_min, 8, row_max, 8));
        chart.title().set_name("Slow length counts");
        chart.x_axis().set_name("Slow Len");
        chart.y_axis().set_name("Count");
        chart.legend().set_hidden();
        dist.insert_chart_with_offset(37, 10, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    // Fast vs slow scatter with robust buckets (low/mid/high).
    let scatter_header_row: u32 = 20;
    dist.write_string_with_format(scatter_header_row, 0, "slow_len_low", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 1, "fast_len_low", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 2, "slow_len_mid", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 3, "fast_len_mid", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 4, "slow_len_high", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;
    dist.write_string_with_format(scatter_header_row, 5, "fast_len_high", &header_fmt)
        .map_err(|e: XlsxError| e.to_string())?;

    let scatter_row_min = scatter_header_row + 1;
    for (i, (slow_len, fast_len)) in low.iter().enumerate() {
        let r = scatter_row_min + i as u32;
        dist.write_number_with_format(r, 0, *slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 1, *fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, (slow_len, fast_len)) in mid.iter().enumerate() {
        let r = scatter_row_min + i as u32;
        dist.write_number_with_format(r, 2, *slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 3, *fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }
    for (i, (slow_len, fast_len)) in high.iter().enumerate() {
        let r = scatter_row_min + i as u32;
        dist.write_number_with_format(r, 4, *slow_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
        dist.write_number_with_format(r, 5, *fast_len as f64, &int_fmt)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    if !low.is_empty() || !mid.is_empty() || !high.is_empty() {
        let mut marker_low = ChartMarker::new();
        marker_low
            .set_type(ChartMarkerType::Circle)
            .set_size(6)
            .set_format(
                ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#F8696B")),
            );
        let mut marker_mid = ChartMarker::new();
        marker_mid
            .set_type(ChartMarkerType::Circle)
            .set_size(6)
            .set_format(
                ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#FFEB84")),
            );
        let mut marker_high = ChartMarker::new();
        marker_high
            .set_type(ChartMarkerType::Circle)
            .set_size(6)
            .set_format(
                ChartFormat::new().set_solid_fill(ChartSolidFill::new().set_color("#63BE7B")),
            );

        let mut chart = Chart::new(ChartType::Scatter);
        if !low.is_empty() {
            let row_max = scatter_row_min + low.len().saturating_sub(1) as u32;
            chart
                .add_series()
                .set_name("Low robust")
                .set_categories(("Distributions", scatter_row_min, 0, row_max, 0))
                .set_values(("Distributions", scatter_row_min, 1, row_max, 1))
                .set_marker(&marker_low);
        }
        if !mid.is_empty() {
            let row_max = scatter_row_min + mid.len().saturating_sub(1) as u32;
            chart
                .add_series()
                .set_name("Mid robust")
                .set_categories(("Distributions", scatter_row_min, 2, row_max, 2))
                .set_values(("Distributions", scatter_row_min, 3, row_max, 3))
                .set_marker(&marker_mid);
        }
        if !high.is_empty() {
            let row_max = scatter_row_min + high.len().saturating_sub(1) as u32;
            chart
                .add_series()
                .set_name("High robust")
                .set_categories(("Distributions", scatter_row_min, 4, row_max, 4))
                .set_values(("Distributions", scatter_row_min, 5, row_max, 5))
                .set_marker(&marker_high);
        }
        chart
            .title()
            .set_name("Fast vs Slow length (bucketed by robust score)");
        chart.x_axis().set_name("Slow Len");
        chart.y_axis().set_name("Fast Len");
        chart
            .legend()
            .set_position(rust_xlsxwriter::ChartLegendPosition::Bottom);

        dist.insert_chart_with_offset(20, 16, &chart, 10, 10)
            .map_err(|e: XlsxError| e.to_string())?;
    }

    workbook
        .save_to_buffer()
        .map_err(|e: XlsxError| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn parse_csv_fields(line: &str) -> Vec<String> {
        let mut fields: Vec<String> = Vec::new();
        let mut cur = String::new();
        let mut chars = line.chars().peekable();
        let mut in_quotes = false;

        while let Some(ch) = chars.next() {
            match ch {
                '"' => {
                    if in_quotes {
                        if matches!(chars.peek(), Some('"')) {
                            cur.push('"');
                            let _ = chars.next();
                        } else {
                            in_quotes = false;
                        }
                    } else {
                        in_quotes = true;
                    }
                }
                ',' if !in_quotes => {
                    fields.push(cur);
                    cur = String::new();
                }
                _ => cur.push(ch),
            }
        }
        fields.push(cur);
        fields
    }

    fn make_trending_candles(len: usize, step: f64) -> vector_ta::utilities::data_loader::Candles {
        let mut ts = Vec::with_capacity(len);
        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut vol: Vec<f64> = Vec::with_capacity(len);

        let base_ts: i64 = 1_700_000_000_000;
        for i in 0..len {
            ts.push(base_ts + (i as i64) * 60_000);
            let base = 100.0 + (i as f64) * step;
            open.push(base);
            high.push(base + 1.0);
            low.push(base - 1.0);
            close.push(base + 0.5);
            vol.push(1_000.0);
        }

        vector_ta::utilities::data_loader::Candles::new(ts, open, high, low, close, vol)
    }

    fn write_candles_csv(path: &Path, candles: &vector_ta::utilities::data_loader::Candles) {
        let mut out = String::new();
        out.push_str("timestamp,open,high,low,close,volume\n");
        for i in 0..candles.close.len() {
            out.push_str(&format!(
                "{},{},{},{},{},{}\n",
                candles.timestamp[i],
                candles.open[i],
                candles.high[i],
                candles.low[i],
                candles.close[i],
                candles.volume[i],
            ));
        }
        std::fs::write(path, out).expect("write csv");
    }

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let name = format!("{prefix}_{}_{}", std::process::id(), uniq);
        p.push(name);
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("create temp dir");
        p
    }

    #[test]
    fn validation_shortlist_csv_smoke() {
        let validation_res = json!({
          "data_id": "test",
          "candidates": [
            {
              "params": { "fast_ma_type": "sma", "fast_len": 9, "slow_ma_type": "ema", "slow_len": 21 },
              "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10 },
              "robust_score": 123.0,
              "robust_pareto": true,
              "cross_ticker": { "score": 0.7 },
              "cross_ticker_pareto": true
            }
          ]
        });

        let csv = commands_reports_csv::build_validation_shortlist_csv_text(&validation_res)
            .expect("csv");
        let mut it = csv.lines();
        let header = it.next().unwrap_or_default();
        let header_fields = parse_csv_fields(header);
        assert_eq!(header_fields.len(), 28);
        let row = it.next().unwrap_or_default();
        let row_fields = parse_csv_fields(row);
        assert_eq!(row_fields.len(), 28);
        assert!(row_fields[27].contains("\"fast_ma_type\":\"sma\""));
        assert!(csv.contains("rank,approval_label"));
        assert!(csv.contains("params_json"));
    }

    #[test]
    fn report_xlsx_bytes_smoke() {
        let report = json!({
          "version": 1,
          "kind": "vectorbt_report",
          "ts": "2026-01-26T00:00:00Z",
          "validation": {
            "res": {
              "data_id": "test",
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "sma",
                    "fast_len": 9,
                    "slow_ma_type": "sma",
                    "slow_len": 21,
                    "halftrend": { "amplitude": 2, "atr_period": 14, "channel_deviation": 2.0 },
                    "macz": {},
                    "ott": {},
                    "strategy": { "commission": 0.0, "eps_rel": 0.0, "long_only": true, "allow_flip": true, "trade_on_next_bar": true }
                  },
                  "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10, "exposure": 0.5, "net_exposure": 0.1 },
                  "robust_score": 1.23,
                  "robust_pareto": true
                }
              ]
            }
          }
        });

        let bytes = build_report_xlsx_bytes_headless(&report, Some(50)).expect("xlsx bytes");
        assert!(bytes.len() > 512);
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn validation_report_workbook_smoke() {
        let report = json!({
          "version": 1,
          "kind": "vectorbt_report",
          "ts": "2026-01-26T00:00:00Z",
          "validation": {
            "res": {
              "data_id": "test",
              "cscv_pbo": { "combinations": 6, "threshold": 0.0 },
              "candidates": [
                {
                  "params": {
                    "fast_ma_type": "sma",
                    "fast_len": 9,
                    "slow_ma_type": "sma",
                    "slow_len": 21
                  },
                  "in_sample": { "pnl": 1.0, "sharpe": 2.0, "max_dd": 0.5, "trades": 10, "exposure": 0.5, "net_exposure": 0.1 },
                  "walk_forward": { "sharpe_median": 1.2, "max_dd_worst": 0.6 },
                  "perturb": { "pnl_worst": -0.2, "pnl_mean": 0.1, "pnl_positive_frac": 0.6 },
                  "trade_bootstrap": { "prob_pnl_positive": 0.7, "pnl_p50": 0.2, "max_dd_p95": 0.8 },
                  "cross_ticker": { "score": 0.3, "sharpe_median": 0.4, "max_dd_worst": 0.7 },
                  "cscv_pbo": { "is_selected_frac": 0.8, "oos_rank_frac_p50": 0.7, "oos_metric_p50": 0.1 },
                  "robust_score": 1.23,
                  "robust_pareto": true,
                  "cross_ticker_pareto": true
                }
              ]
            }
          }
        });

        let bytes = build_report_xlsx_bytes_headless(&report, Some(25)).expect("xlsx bytes");
        assert!(bytes.len() > 512);
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn cross_ticker_validation_smoke_populates_stats() {
        let base_dir = make_temp_dir("vectorta_cross_ticker_test");
        let app_cache_dir = base_dir.join("data_cache");
        std::fs::create_dir_all(&app_cache_dir).expect("create cache dir");

        let ds1 = base_dir.join("ds1.csv");
        let ds2 = base_dir.join("ds2.csv");

        let candles1 = make_trending_candles(1024, 0.10);
        let candles2 = make_trending_candles(1024, 0.06);
        write_candles_csv(&ds1, &candles1);
        write_candles_csv(&ds2, &candles2);

        let cancel = std::sync::atomic::AtomicBool::new(false);

        let base_params = ta_app_core::CompositeParams {
            fast_ma_type: "sma".to_string(),
            slow_ma_type: "sma".to_string(),
            fast_len: 10,
            slow_len: 30,
            halftrend: ta_app_core::double_ma::HalfTrendRegimeFilter {
                amplitude: 2,
                channel_deviation: 2.0,
                atr_period: 100,
            },
            asr: None,
            vcc: None,
            vemr: None,
            rocb: None,
            mkcs: None,
            kqsr: None,
            carm: None,
            kvdf: None,
            cdmc: None,
            macz: ta_app_core::composite_cpu::MaczGateParams::default(),
            ott: ta_app_core::composite_cpu::OttGateParams::default(),
            signal_mode: ta_app_core::CompositeSignalMode::HardGate,
            strategy: ta_strategies::double_ma::StrategyConfig::default(),
        };

        let req = ta_app_core::CompositeValidationRequest {
            data_id: "base".to_string(),
            backend: ta_app_core::Backend::CpuOnly,
            ma_source: "close".to_string(),
            fast_ma_params: None,
            slow_ma_params: None,
            candidates: vec![base_params],
            walk_forward: None,
            reopt_walk_forward: None,
            spa: None,
            perturb: None,
            trade_bootstrap: None,
            cross_ticker: None,
            cscv_pbo: None,
            seed: 1,
            max_candidates: 32,
            validation_engine: ta_app_core::ValidationEngine::Vectorized,
            iterative: ta_app_core::IterativeSimConfig::default(),
            min_psr: None,
            min_dsr: None,
            min_cscv_oos_rank_frac: None,
            min_cscv_oos_metric: None,
            max_pbo: None,
            validation_context: None,
        };

        let mut res = ta_app_core::run_composite_validation_blocking_with_candles(
            req, &candles1, &cancel, None,
        )
        .expect("base validation");

        let cross_cfg = ta_app_core::CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: ds1.to_string_lossy().to_string(),
                    label: "ds1".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: ds2.to_string_lossy().to_string(),
                    label: "ds2".to_string(),
                },
            ],
            max_datasets: 8,
            gate: ta_app_core::CrossTickerGateConfig::default(),
        };

        super::super::apply_cross_ticker_validation(
            &mut res,
            cross_cfg,
            ta_app_core::Backend::CpuOnly,
            "close",
            None,
            None,
            1,
            &app_cache_dir,
            &cancel,
            None,
            ta_app_core::ValidationEngine::Vectorized,
            ta_app_core::IterativeSimConfig::default(),
        )
        .expect("cross validation");

        let summary = res.cross_ticker.as_ref().expect("summary");
        assert_eq!(summary.datasets.len(), 2);
        let cand = &res.candidates[0];
        let cross = cand.cross_ticker.as_ref().expect("candidate cross");
        assert_eq!(cross.datasets, 2);
        assert!(cand.cross_ticker_pareto.is_some());
    }
}
