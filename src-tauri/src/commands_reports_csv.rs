use serde_json::Value;

fn csv_escape(v: &str) -> String {
    if v.contains(',') || v.contains('"') || v.contains('\n') || v.contains('\r') {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

pub(crate) fn build_validation_shortlist_csv_blocking(
    validation_res: Value,
) -> Result<String, String> {
    build_validation_shortlist_csv_text(&validation_res)
}

pub(super) fn build_validation_shortlist_csv_text(
    validation_res: &Value,
) -> Result<String, String> {
    const CSCV_RANK_FRAC_MIN: f64 = 0.55;

    let candidates = validation_res
        .get("candidates")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing validation_res.candidates".to_string())?;

    let has_cross = candidates
        .iter()
        .any(|c| c.get("cross_ticker").and_then(|v| v.as_object()).is_some());

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

    let base_rows: Vec<&Value> = if strict.is_empty() {
        candidates.iter().collect()
    } else {
        strict
    };

    let mut rows: Vec<&Value> = if has_cscv {
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

    if rows.is_empty() {
        return Err("validation_res has no candidates".to_string());
    }

    let mut lines: Vec<String> = Vec::with_capacity(rows.len().saturating_add(1));
    lines.push("rank,fast_ma,fast_len,slow_ma,slow_len,robust_score,robust_pareto,cross_score,cross_pareto,cscv_selected_frac,cscv_oos_rank_frac_p50,cscv_oos_sharpe_p50,psr,dsr,in_sample_pnl,in_sample_sharpe,in_sample_max_dd,in_sample_trades,params_json".to_string());

    rows.sort_by(|a, b| {
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

    for (i, cand) in rows.into_iter().enumerate() {
        let params = cand.get("params").and_then(|v| v.as_object());
        let in_sample = cand.get("in_sample").and_then(|v| v.as_object());
        let cross = cand.get("cross_ticker").and_then(|v| v.as_object());
        let cscv = cand.get("cscv_pbo").and_then(|v| v.as_object());

        let fast_ma = params
            .and_then(|p| p.get("fast_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let fast_len = params
            .and_then(|p| p.get("fast_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);
        let slow_ma = params
            .and_then(|p| p.get("slow_ma_type").and_then(|v| v.as_str()))
            .unwrap_or("");
        let slow_len = params
            .and_then(|p| p.get("slow_len").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let robust_score = cand
            .get("robust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let robust_pareto = cand
            .get("robust_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cross_score = cross.and_then(|c| c.get("score").and_then(|v| v.as_f64()));
        let cross_pareto = cand
            .get("cross_ticker_pareto")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let cscv_sel = cscv.and_then(|c| c.get("is_selected_frac").and_then(|v| v.as_f64()));
        let cscv_rank = cscv.and_then(|c| c.get("oos_rank_frac_p50").and_then(|v| v.as_f64()));
        let cscv_sharpe = cscv.and_then(|c| c.get("oos_metric_p50").and_then(|v| v.as_f64()));
        let psr = cand.get("psr").and_then(|v| v.as_f64());
        let dsr = cand.get("dsr").and_then(|v| v.as_f64());

        let pnl = in_sample
            .and_then(|m| m.get("pnl").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let sharpe = in_sample
            .and_then(|m| m.get("sharpe").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let max_dd = in_sample
            .and_then(|m| m.get("max_dd").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let trades = in_sample
            .and_then(|m| m.get("trades").and_then(|v| v.as_u64()))
            .unwrap_or(0);

        let params_json = cand.get("params").cloned().unwrap_or(Value::Null);
        let params_json_str =
            serde_json::to_string(&params_json).unwrap_or_else(|_| "{}".to_string());

        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            i.saturating_add(1),
            csv_escape(fast_ma),
            fast_len,
            csv_escape(slow_ma),
            slow_len,
            robust_score,
            if robust_pareto { 1 } else { 0 },
            cross_score
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            if cross_pareto { 1 } else { 0 },
            cscv_sel
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            cscv_rank
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            cscv_sharpe
                .map(|x| x.to_string())
                .unwrap_or_else(|| "".to_string()),
            psr.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
            dsr.map(|x| x.to_string()).unwrap_or_else(|| "".to_string()),
            pnl,
            sharpe,
            max_dd,
            trades,
            csv_escape(&params_json_str),
        ));
    }

    Ok(lines.join("\n"))
}
