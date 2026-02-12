use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use ta_app_core::composite_validate::CompositeValidationCandidateResult;
use ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult;
use ta_app_core::{CrossTickerConfig, CrossTickerStats};
use ta_strategies::double_ma::Metrics;

#[derive(Clone, Default)]
pub struct CrossTickerAcc {
    pub sharpes: Vec<f64>,
    pub dds: Vec<f64>,
    pub pnls: Vec<f64>,
    pub trades: Vec<u32>,
    pub pnl_pos: usize,
}

pub trait CrossTickerCandidate {
    type Params: Serialize;

    fn cross_ticker(&self) -> Option<&CrossTickerStats>;
    fn set_cross_ticker(&mut self, stats: CrossTickerStats);
    fn set_cross_ticker_pareto(&mut self, is_pareto: bool);
    fn robust_score(&self) -> f64;
    fn set_robust_score(&mut self, score: f64);
}

impl CrossTickerCandidate for CompositeValidationCandidateResult {
    type Params = ta_app_core::CompositeParams;

    fn cross_ticker(&self) -> Option<&CrossTickerStats> {
        self.cross_ticker.as_ref()
    }

    fn set_cross_ticker(&mut self, stats: CrossTickerStats) {
        self.cross_ticker = Some(stats);
    }

    fn set_cross_ticker_pareto(&mut self, is_pareto: bool) {
        self.cross_ticker_pareto = Some(is_pareto);
    }

    fn robust_score(&self) -> f64 {
        self.robust_score
    }

    fn set_robust_score(&mut self, score: f64) {
        self.robust_score = score;
    }
}

impl CrossTickerCandidate for DoubleMaValidationCandidateResult {
    type Params = ta_app_core::DoubleMaValidationParams;

    fn cross_ticker(&self) -> Option<&CrossTickerStats> {
        self.cross_ticker.as_ref()
    }

    fn set_cross_ticker(&mut self, stats: CrossTickerStats) {
        self.cross_ticker = Some(stats);
    }

    fn set_cross_ticker_pareto(&mut self, is_pareto: bool) {
        self.cross_ticker_pareto = Some(is_pareto);
    }

    fn robust_score(&self) -> f64 {
        self.robust_score
    }

    fn set_robust_score(&mut self, score: f64) {
        self.robust_score = score;
    }
}

pub fn sanitize_cross_ticker_config(cross_cfg: &mut CrossTickerConfig) {
    cross_cfg.datasets.retain(|d| !d.csv_path.trim().is_empty());
    let mut seen: HashSet<String> = HashSet::with_capacity(cross_cfg.datasets.len() * 2);
    cross_cfg
        .datasets
        .retain(|d| seen.insert(d.csv_path.trim().to_string()));
}

pub fn dataset_label(csv_path: &str, label: &str) -> String {
    let l = label.trim();
    if !l.is_empty() {
        return l.to_string();
    }
    Path::new(csv_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("dataset")
        .to_string()
}

fn canonicalize_json_value(v: &Value) -> Value {
    match v {
        Value::Array(xs) => Value::Array(xs.iter().map(canonicalize_json_value).collect()),
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                if let Some(v2) = map.get(k) {
                    out.insert(k.clone(), canonicalize_json_value(v2));
                }
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}

pub fn candidate_key_canonical<T: Serialize>(params: &T) -> Result<String, String> {
    let v = serde_json::to_value(params).map_err(|e| e.to_string())?;
    serde_json::to_string(&canonicalize_json_value(&v)).map_err(|e| e.to_string())
}

pub fn push_metrics(acc: &mut CrossTickerAcc, m: &Metrics) {
    acc.sharpes.push(m.sharpe);
    acc.dds.push(m.max_dd);
    acc.pnls.push(m.pnl);
    acc.trades.push(m.trades);
    if m.pnl > 0.0 {
        acc.pnl_pos = acc.pnl_pos.saturating_add(1);
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return if sorted[0].is_finite() { sorted[0] } else { 0.0 };
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    let v = sorted[idx.min(sorted.len() - 1)];
    if v.is_finite() { v } else { 0.0 }
}

fn median(sorted: &[f64]) -> f64 {
    quantile(sorted, 0.5)
}

fn worst_min(xs: &[f64]) -> f64 {
    let v = xs
        .iter()
        .copied()
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a });
    if v.is_finite() { v } else { 0.0 }
}

fn worst_max(xs: &[f64]) -> f64 {
    let v = xs
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a });
    if v.is_finite() { v } else { 0.0 }
}

fn median_u32(xs: &[u32]) -> u32 {
    if xs.is_empty() {
        return 0;
    }
    let mut ys = xs.to_vec();
    ys.sort_unstable();
    ys[ys.len() / 2]
}

fn worst_min_u32(xs: &[u32]) -> u32 {
    xs.iter().copied().min().unwrap_or(0)
}

pub fn finalize_cross_ticker_candidates<C: CrossTickerCandidate>(
    candidates: &mut [C],
    mut accs: Vec<CrossTickerAcc>,
) {
    for (i, cand) in candidates.iter_mut().enumerate() {
        let Some(acc) = accs.get_mut(i) else {
            continue;
        };
        let ds_n = acc.sharpes.len();
        if ds_n == 0 {
            continue;
        }

        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));

        let sharpe_median = median(&acc.sharpes);
        let max_dd_worst = worst_max(&acc.dds);
        let score = sharpe_median - 2.0 * max_dd_worst;
        cand.set_cross_ticker(CrossTickerStats {
            datasets: ds_n,
            sharpe_median,
            sharpe_worst: worst_min(&acc.sharpes),
            max_dd_median: median(&acc.dds),
            max_dd_worst,
            pnl_median: median(&acc.pnls),
            pnl_worst: worst_min(&acc.pnls),
            trades_median: median_u32(&acc.trades),
            trades_worst: worst_min_u32(&acc.trades),
            pnl_positive_frac: acc.pnl_pos as f64 / ds_n as f64,
            score,
        });
        if score.is_finite() {
            let next = if cand.robust_score().is_finite() {
                cand.robust_score().min(score)
            } else {
                score
            };
            cand.set_robust_score(next);
        }
    }

    for i in 0..candidates.len() {
        let Some(si) = candidates[i].cross_ticker() else {
            continue;
        };
        let xi = si.sharpe_median;
        let yi = si.max_dd_worst;
        let mut dominated = false;
        for j in 0..candidates.len() {
            if i == j {
                continue;
            }
            let Some(sj) = candidates[j].cross_ticker() else {
                continue;
            };
            let xj = sj.sharpe_median;
            let yj = sj.max_dd_worst;
            if xj >= xi && yj <= yi && (xj > xi || yj < yi) {
                dominated = true;
                break;
            }
        }
        candidates[i].set_cross_ticker_pareto(!dominated);
    }

    candidates.sort_by(|a, b| b.robust_score().total_cmp(&a.robust_score()));
}
