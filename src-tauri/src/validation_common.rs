use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use ta_app_core::composite_validate::CompositeValidationCandidateResult;
use ta_app_core::double_ma_validate::DoubleMaValidationCandidateResult;
use ta_app_core::{CrossTickerConfig, CrossTickerGateConfig, CrossTickerStats};
use ta_strategies::double_ma::Metrics;

#[derive(Clone, Default)]
pub struct CrossTickerAcc {
    pub sharpes: Vec<f64>,
    pub dds: Vec<f64>,
    pub pnls: Vec<f64>,
    pub trades: Vec<u32>,
    pub pnl_pos: usize,
}

impl CrossTickerAcc {
    pub fn with_capacity(datasets: usize) -> Self {
        Self {
            sharpes: Vec::with_capacity(datasets),
            dds: Vec::with_capacity(datasets),
            pnls: Vec::with_capacity(datasets),
            trades: Vec::with_capacity(datasets),
            pnl_pos: 0,
        }
    }
}

pub trait CrossTickerCandidate {
    type Params: Serialize;

    fn params(&self) -> &Self::Params;
    fn cross_ticker_metrics(&self) -> &Metrics;
    fn validation_passed(&self) -> bool;
    fn cross_ticker(&self) -> Option<&CrossTickerStats>;
    fn cross_ticker_pareto(&self) -> Option<bool>;
    fn set_cross_ticker(&mut self, stats: CrossTickerStats);
    fn set_cross_ticker_pareto(&mut self, is_pareto: bool);
    fn robust_score(&self) -> f64;
    fn set_robust_score(&mut self, score: f64);
}

pub fn normalize_path_key(raw: &str) -> String {
    let trimmed = raw.trim();
    #[cfg(target_os = "windows")]
    {
        trimmed.replace('/', "\\").to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        trimmed.to_string()
    }
}

impl CrossTickerCandidate for CompositeValidationCandidateResult {
    type Params = ta_app_core::CompositeParams;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn cross_ticker_metrics(&self) -> &Metrics {
        self.iterative_in_sample.as_ref().unwrap_or(&self.in_sample)
    }

    fn validation_passed(&self) -> bool {
        self.validation_verdict.passed
    }

    fn cross_ticker(&self) -> Option<&CrossTickerStats> {
        self.cross_ticker.as_ref()
    }

    fn cross_ticker_pareto(&self) -> Option<bool> {
        self.cross_ticker_pareto
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

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn cross_ticker_metrics(&self) -> &Metrics {
        self.iterative_in_sample.as_ref().unwrap_or(&self.in_sample)
    }

    fn validation_passed(&self) -> bool {
        self.validation_verdict.passed
    }

    fn cross_ticker(&self) -> Option<&CrossTickerStats> {
        self.cross_ticker.as_ref()
    }

    fn cross_ticker_pareto(&self) -> Option<bool> {
        self.cross_ticker_pareto
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
        .retain(|d| seen.insert(normalize_path_key(&d.csv_path)));
    cross_cfg.gate.min_datasets = cross_cfg.gate.min_datasets.max(1);
    cross_cfg.gate.min_score = if cross_cfg.gate.min_score.is_finite() {
        cross_cfg.gate.min_score
    } else {
        0.0
    };
    cross_cfg.gate.min_sharpe_median = if cross_cfg.gate.min_sharpe_median.is_finite() {
        cross_cfg.gate.min_sharpe_median
    } else {
        0.0
    };
    cross_cfg.gate.min_pnl_positive_frac = if cross_cfg.gate.min_pnl_positive_frac.is_finite() {
        cross_cfg.gate.min_pnl_positive_frac.clamp(0.0, 1.0)
    } else {
        0.5
    };
    cross_cfg.gate.max_dd_worst = cross_cfg.gate.max_dd_worst.and_then(|v| {
        if v.is_finite() {
            Some(v.max(0.0))
        } else {
            None
        }
    });
}

pub fn cross_ticker_passes_gate(
    stats: &CrossTickerStats,
    cross_ticker_pareto: Option<bool>,
    gate: &CrossTickerGateConfig,
) -> bool {
    if !gate.enabled {
        return true;
    }
    if gate.require_pareto && !cross_ticker_pareto.unwrap_or(false) {
        return false;
    }
    if stats.datasets < gate.min_datasets {
        return false;
    }
    if stats.score < gate.min_score {
        return false;
    }
    if stats.sharpe_median < gate.min_sharpe_median {
        return false;
    }
    if stats.pnl_positive_frac < gate.min_pnl_positive_frac {
        return false;
    }
    if let Some(max_dd_worst) = gate.max_dd_worst {
        if stats.max_dd_worst > max_dd_worst {
            return false;
        }
    }
    true
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
        return if sorted[0].is_finite() {
            sorted[0]
        } else {
            0.0
        };
    }
    let qq = q.clamp(0.0, 1.0);
    let idx = (qq * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    let v = sorted[idx.min(sorted.len() - 1)];
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn median(sorted: &[f64]) -> f64 {
    quantile(sorted, 0.5)
}

fn worst_min(xs: &[f64]) -> f64 {
    let v = xs
        .iter()
        .copied()
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a });
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn worst_max(xs: &[f64]) -> f64 {
    let v = xs
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |a, b| if b > a { b } else { a });
    if v.is_finite() {
        v
    } else {
        0.0
    }
}

fn median_u32_sorted(xs: &[u32]) -> u32 {
    if xs.is_empty() {
        return 0;
    }
    xs[xs.len() / 2]
}

fn worst_min_u32(xs: &[u32]) -> u32 {
    xs.iter().copied().min().unwrap_or(0)
}

pub fn finalize_cross_ticker_candidates<C: CrossTickerCandidate>(
    candidates: &mut [C],
    mut accs: Vec<CrossTickerAcc>,
) {
    #[derive(Clone, Copy)]
    struct CrossTickerParetoPoint {
        sharpe_median: f64,
        max_dd_worst: f64,
    }

    for (i, cand) in candidates.iter_mut().enumerate() {
        let Some(acc) = accs.get_mut(i) else {
            continue;
        };
        let ds_n = acc.sharpes.len();
        if ds_n == 0 {
            continue;
        }
        debug_assert_eq!(
            acc.dds.len(),
            ds_n,
            "cross-ticker drawdown accumulator length must match sharpe length"
        );
        debug_assert_eq!(
            acc.pnls.len(),
            ds_n,
            "cross-ticker pnl accumulator length must match sharpe length"
        );
        debug_assert_eq!(
            acc.trades.len(),
            ds_n,
            "cross-ticker trades accumulator length must match sharpe length"
        );

        acc.sharpes.sort_by(|a, b| a.total_cmp(b));
        acc.dds.sort_by(|a, b| a.total_cmp(b));
        acc.pnls.sort_by(|a, b| a.total_cmp(b));
        acc.trades.sort_unstable();

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
            trades_median: median_u32_sorted(&acc.trades),
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

    let pareto_points: Vec<Option<CrossTickerParetoPoint>> = candidates
        .iter()
        .map(|cand| {
            cand.cross_ticker().map(|stats| CrossTickerParetoPoint {
                sharpe_median: stats.sharpe_median,
                max_dd_worst: stats.max_dd_worst,
            })
        })
        .collect();

    for i in 0..candidates.len() {
        let Some(point_i) = pareto_points[i] else {
            continue;
        };
        let mut dominated = false;
        for j in 0..candidates.len() {
            if i == j {
                continue;
            }
            let Some(point_j) = pareto_points[j] else {
                continue;
            };
            if point_j.sharpe_median >= point_i.sharpe_median
                && point_j.max_dd_worst <= point_i.max_dd_worst
                && (point_j.sharpe_median > point_i.sharpe_median
                    || point_j.max_dd_worst < point_i.max_dd_worst)
            {
                dominated = true;
                break;
            }
        }
        candidates[i].set_cross_ticker_pareto(!dominated);
    }

    candidates.sort_by(|a, b| b.robust_score().total_cmp(&a.robust_score()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use ta_app_core::CrossTickerStats;
    use ta_strategies::double_ma::Metrics;

    #[test]
    fn sanitize_cross_ticker_config_clamps_gate_values() {
        let mut cfg = CrossTickerConfig {
            datasets: vec![ta_app_core::CrossTickerDataset {
                csv_path: "  a.csv ".to_string(),
                label: String::new(),
            }],
            max_datasets: 4,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: CrossTickerGateConfig {
                enabled: true,
                require_pareto: true,
                min_score: f64::NAN,
                min_pnl_positive_frac: 2.0,
                min_sharpe_median: f64::NAN,
                max_dd_worst: Some(f64::NAN),
                min_datasets: 0,
            },
        };

        sanitize_cross_ticker_config(&mut cfg);
        assert_eq!(cfg.gate.min_datasets, 1);
        assert_eq!(cfg.gate.min_score, 0.0);
        assert_eq!(cfg.gate.min_sharpe_median, 0.0);
        assert_eq!(cfg.gate.min_pnl_positive_frac, 1.0);
        assert_eq!(cfg.gate.max_dd_worst, None);
    }

    #[test]
    fn sanitize_cross_ticker_config_dedupes_normalized_paths() {
        let mut cfg = CrossTickerConfig {
            datasets: vec![
                ta_app_core::CrossTickerDataset {
                    csv_path: "C:/data/AAPL.csv".to_string(),
                    label: "first".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: " c:\\DATA\\aapl.csv ".to_string(),
                    label: "second".to_string(),
                },
                ta_app_core::CrossTickerDataset {
                    csv_path: "D:/data/MSFT.csv".to_string(),
                    label: "third".to_string(),
                },
            ],
            max_datasets: 4,
            mode: ta_app_core::CrossTickerMode::Lightweight,
            gate: CrossTickerGateConfig::default(),
        };

        sanitize_cross_ticker_config(&mut cfg);

        #[cfg(target_os = "windows")]
        {
            assert_eq!(cfg.datasets.len(), 2);
            assert_eq!(cfg.datasets[0].label, "first");
            assert_eq!(cfg.datasets[1].label, "third");
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(cfg.datasets.len(), 3);
            assert_eq!(cfg.datasets[0].label, "first");
            assert_eq!(cfg.datasets[1].label, "second");
            assert_eq!(cfg.datasets[2].label, "third");
        }
    }

    #[test]
    fn normalize_path_key_is_platform_aware() {
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                normalize_path_key(" C:/Data/AAPL.csv "),
                "c:\\data\\aapl.csv"
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(normalize_path_key(" /Data/AAPL.csv "), "/Data/AAPL.csv");
            assert_ne!(
                normalize_path_key("/Data/AAPL.csv"),
                normalize_path_key("/data/aapl.csv")
            );
        }
    }

    #[test]
    fn quantile_and_median_preserve_current_nearest_rank_behavior() {
        let sorted = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile(&sorted, 0.0), 1.0);
        assert_eq!(quantile(&sorted, 0.5), 3.0);
        assert_eq!(quantile(&sorted, f64::NAN), 1.0);
        assert_eq!(median(&sorted), 3.0);
    }

    #[test]
    fn cross_ticker_gate_enforces_score_and_pareto() {
        let stats = CrossTickerStats {
            datasets: 3,
            sharpe_median: 0.8,
            sharpe_worst: 0.2,
            max_dd_median: 0.25,
            max_dd_worst: 0.40,
            pnl_median: 1.0,
            pnl_worst: 0.2,
            trades_median: 12,
            trades_worst: 5,
            pnl_positive_frac: 0.67,
            score: 0.1,
        };
        let gate = CrossTickerGateConfig::default();
        assert!(cross_ticker_passes_gate(&stats, Some(true), &gate));
        assert!(!cross_ticker_passes_gate(&stats, Some(false), &gate));

        let mut gate_tight = gate.clone();
        gate_tight.min_score = 0.2;
        assert!(!cross_ticker_passes_gate(&stats, Some(true), &gate_tight));
    }

    #[derive(Clone, Serialize)]
    struct TestParams {
        id: u32,
    }

    #[derive(Clone)]
    struct TestCandidate {
        params: TestParams,
        in_sample: Metrics,
        cross_ticker: Option<CrossTickerStats>,
        cross_ticker_pareto: Option<bool>,
        robust_score: f64,
    }

    impl CrossTickerCandidate for TestCandidate {
        type Params = TestParams;

        fn params(&self) -> &Self::Params {
            &self.params
        }

        fn cross_ticker_metrics(&self) -> &Metrics {
            &self.in_sample
        }

        fn validation_passed(&self) -> bool {
            true
        }

        fn cross_ticker(&self) -> Option<&CrossTickerStats> {
            self.cross_ticker.as_ref()
        }

        fn cross_ticker_pareto(&self) -> Option<bool> {
            self.cross_ticker_pareto
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

    #[test]
    fn finalize_cross_ticker_folds_robust_score_and_sorts_descending() {
        let mut candidates = vec![
            TestCandidate {
                params: TestParams { id: 1 },
                in_sample: Metrics::default(),
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: 1.0,
            },
            TestCandidate {
                params: TestParams { id: 2 },
                in_sample: Metrics::default(),
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: 0.5,
            },
        ];
        let accs = vec![
            CrossTickerAcc {
                sharpes: vec![0.4, 0.4],
                dds: vec![0.1, 0.1],
                pnls: vec![0.1, 0.1],
                trades: vec![10, 10],
                pnl_pos: 2,
            },
            CrossTickerAcc {
                sharpes: vec![1.2, 1.2],
                dds: vec![0.2, 0.2],
                pnls: vec![0.1, 0.1],
                trades: vec![10, 10],
                pnl_pos: 2,
            },
        ];

        finalize_cross_ticker_candidates(&mut candidates, accs);

        // id=1 gets folded to 0.2 (min(1.0, 0.4 - 2*0.1)),
        // id=2 remains 0.5 (min(0.5, 1.2 - 2*0.2) = 0.5),
        // so descending sort should place id=2 first.
        assert_eq!(candidates[0].params.id, 2);
        assert_eq!(candidates[1].params.id, 1);
        assert!((candidates[0].robust_score - 0.5).abs() < 1e-12);
        assert!((candidates[1].robust_score - 0.2).abs() < 1e-12);
        assert!(candidates[0].cross_ticker_pareto.is_some());
        assert!(candidates[1].cross_ticker_pareto.is_some());
    }

    #[test]
    fn finalize_cross_ticker_candidates_keeps_candidates_without_accumulators_unset() {
        let mut candidates = vec![
            TestCandidate {
                params: TestParams { id: 1 },
                in_sample: Metrics::default(),
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: 1.0,
            },
            TestCandidate {
                params: TestParams { id: 2 },
                in_sample: Metrics::default(),
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: 0.5,
            },
        ];
        let accs = vec![CrossTickerAcc {
            sharpes: vec![0.4, 0.4],
            dds: vec![0.1, 0.1],
            pnls: vec![0.1, 0.1],
            trades: vec![10, 10],
            pnl_pos: 2,
        }];

        finalize_cross_ticker_candidates(&mut candidates, accs);

        assert!(candidates
            .iter()
            .any(|cand| cand.params.id == 1 && cand.cross_ticker.is_some()));
        assert!(candidates
            .iter()
            .any(|cand| cand.params.id == 2 && cand.cross_ticker.is_none()));
    }

    #[test]
    fn finalize_cross_ticker_candidates_preserves_stable_order_for_equal_scores() {
        let mut candidates = vec![
            TestCandidate {
                params: TestParams { id: 1 },
                in_sample: Metrics::default(),
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: f64::NAN,
            },
            TestCandidate {
                params: TestParams { id: 2 },
                in_sample: Metrics::default(),
                cross_ticker: None,
                cross_ticker_pareto: None,
                robust_score: f64::NAN,
            },
        ];
        let accs = vec![
            CrossTickerAcc {
                sharpes: vec![1.0, 1.0],
                dds: vec![0.2, 0.2],
                pnls: vec![0.1, 0.1],
                trades: vec![10, 10],
                pnl_pos: 2,
            },
            CrossTickerAcc {
                sharpes: vec![1.0, 1.0],
                dds: vec![0.2, 0.2],
                pnls: vec![0.1, 0.1],
                trades: vec![10, 10],
                pnl_pos: 2,
            },
        ];

        finalize_cross_ticker_candidates(&mut candidates, accs);

        assert_eq!(candidates[0].params.id, 1);
        assert_eq!(candidates[1].params.id, 2);
    }

    #[test]
    fn candidate_key_canonical_is_stable_for_equivalent_nested_json_shapes() {
        let a = serde_json::json!({
            "b": 1,
            "a": {
                "z": true,
                "x": [
                    { "k2": 2, "k1": 1 }
                ]
            }
        });
        let b = serde_json::json!({
            "a": {
                "x": [
                    { "k1": 1, "k2": 2 }
                ],
                "z": true
            },
            "b": 1
        });

        let key_a = candidate_key_canonical(&a).expect("canonical key a");
        let key_b = candidate_key_canonical(&b).expect("canonical key b");

        assert_eq!(key_a, key_b);
        assert_eq!(
            key_a,
            "{\"a\":{\"x\":[{\"k1\":1,\"k2\":2}],\"z\":true},\"b\":1}"
        );
    }
}
