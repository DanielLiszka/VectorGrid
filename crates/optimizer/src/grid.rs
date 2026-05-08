use crate::{
    compare_score_rank_desc, normalize_rank_score, BacktestEngine, ObjectiveKind,
    OptimizationResult,
};
use ta_strategies::double_ma::{DoubleMaParams, Metrics};

fn score(metrics: &Metrics, objective: ObjectiveKind) -> f64 {
    let score = match objective {
        ObjectiveKind::Pnl => metrics.pnl,
        ObjectiveKind::Sharpe => metrics.sharpe,
        ObjectiveKind::Sortino => metrics.sortino,
        ObjectiveKind::Calmar => metrics.calmar,
        ObjectiveKind::Omega => metrics.omega,
        ObjectiveKind::Volatility => -metrics.volatility,
        ObjectiveKind::MaxDrawdown => -metrics.max_dd,
    };
    normalize_rank_score(score)
}

pub fn grid_search<E: BacktestEngine>(
    engine: &E,
    combos: &[DoubleMaParams],
    objective: ObjectiveKind,
    num_candles: usize,
    top_k: usize,
    include_all: bool,
) -> Option<OptimizationResult> {
    if combos.is_empty() {
        return None;
    }
    let metrics = engine.eval_batch(combos);
    if metrics.len() != combos.len() {
        return None;
    }

    let mut best_idx = 0usize;
    let mut best_score = score(&metrics[0], objective);

    let (top, top_scores) = if top_k == 0 {
        for (i, metrics) in metrics.iter().enumerate().skip(1) {
            let s = score(metrics, objective);
            if compare_score_rank_desc(s, i, best_score, best_idx).is_lt() {
                best_score = s;
                best_idx = i;
            }
        }
        (Vec::new(), Vec::new())
    } else {
        let scores: Vec<f64> = metrics.iter().map(|m| score(m, objective)).collect();
        best_score = scores[0];
        for (i, &s) in scores.iter().enumerate().skip(1) {
            if compare_score_rank_desc(s, i, best_score, best_idx).is_lt() {
                best_score = s;
                best_idx = i;
            }
        }
        let keep = top_k.min(metrics.len());
        let mut idx: Vec<usize> = (0..metrics.len()).collect();
        if keep < idx.len() {
            idx.select_nth_unstable_by(keep, |&a, &b| {
                compare_score_rank_desc(scores[a], a, scores[b], b)
            });
            idx.truncate(keep);
        }
        idx.sort_by(|&a, &b| compare_score_rank_desc(scores[a], a, scores[b], b));
        let top_scores = idx.iter().map(|&i| scores[i]).collect();
        let top = idx
            .into_iter()
            .map(|i| (combos[i].clone(), metrics[i].clone()))
            .collect();
        (top, top_scores)
    };

    let best_params = combos[best_idx].clone();
    let best_metrics = metrics[best_idx].clone();
    let all = if include_all {
        Some(combos.iter().cloned().zip(metrics).collect())
    } else {
        None
    };

    Some(OptimizationResult {
        best_params,
        best_metrics,
        best_score: Some(best_score),
        top,
        top_scores: Some(top_scores),
        all,
        all_scores: None,
        heatmap: None,
        num_combos: combos.len(),
        num_candles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::StreamAggregator;

    struct StubEngine {
        metrics: Vec<Metrics>,
    }

    impl BacktestEngine for StubEngine {
        fn eval_batch(&self, _combos: &[DoubleMaParams]) -> Vec<Metrics> {
            self.metrics.clone()
        }
    }

    fn params(fast: u16, slow: u16) -> DoubleMaParams {
        DoubleMaParams {
            fast_len: fast,
            slow_len: slow,
            fast_ma_id: 0,
            slow_ma_id: 0,
        }
    }

    fn metrics_with_sharpe(sharpe: f64) -> Metrics {
        Metrics {
            sharpe,
            ..Metrics::default()
        }
    }

    #[test]
    fn grid_search_clamps_non_finite_scores_like_stream_aggregation() {
        let combos = vec![params(5, 20), params(6, 24), params(8, 30)];
        let engine = StubEngine {
            metrics: vec![
                metrics_with_sharpe(f64::NAN),
                metrics_with_sharpe(1.5),
                metrics_with_sharpe(0.5),
            ],
        };

        let result = grid_search(&engine, &combos, ObjectiveKind::Sharpe, 512, 2, true)
            .expect("grid result");

        assert_eq!(result.best_params.fast_len, 6);
        assert_eq!(result.best_params.slow_len, 24);
        assert_eq!(result.best_score, Some(1.5));
        assert_eq!(result.top.len(), 2);
        assert_eq!(result.top[0].0.fast_len, 6);
        assert_eq!(result.top[1].0.fast_len, 8);
    }

    #[test]
    fn grid_search_matches_stream_best_and_topk_on_shared_inputs() {
        let combos = vec![params(5, 20), params(6, 24), params(8, 30), params(10, 40)];
        let metrics = vec![
            metrics_with_sharpe(0.2),
            metrics_with_sharpe(1.4),
            metrics_with_sharpe(0.8),
            metrics_with_sharpe(1.1),
        ];
        let engine = StubEngine {
            metrics: metrics.clone(),
        };

        let grid = grid_search(&engine, &combos, ObjectiveKind::Sharpe, 512, 3, true)
            .expect("grid result");

        let mut stream = StreamAggregator::new(ObjectiveKind::Sharpe, 3, true, 512);
        for (params, metrics) in combos.iter().cloned().zip(metrics.into_iter()) {
            stream.push(params, metrics);
        }
        let stream = stream.finalize().expect("stream result");

        assert_eq!(grid.best_params.fast_len, stream.best_params.fast_len);
        assert_eq!(grid.best_params.slow_len, stream.best_params.slow_len);
        assert_eq!(grid.best_score, stream.best_score);
        assert_eq!(
            grid.top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            stream
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>()
        );
        assert_eq!(grid.top_scores, stream.top_scores);
        assert_eq!(
            grid.all.as_ref().map(Vec::len),
            stream.all.as_ref().map(Vec::len)
        );
    }

    #[test]
    fn grid_search_uses_negated_risk_scores_for_ranking() {
        let combos = vec![params(5, 20), params(6, 24), params(8, 30)];
        let engine = StubEngine {
            metrics: vec![
                Metrics {
                    volatility: 3.0,
                    ..Metrics::default()
                },
                Metrics {
                    volatility: 1.0,
                    ..Metrics::default()
                },
                Metrics {
                    volatility: 2.0,
                    ..Metrics::default()
                },
            ],
        };

        let result = grid_search(&engine, &combos, ObjectiveKind::Volatility, 512, 3, false)
            .expect("grid result");

        assert_eq!(result.best_params.fast_len, 6);
        assert_eq!(result.best_score, Some(-1.0));
        assert_eq!(result.top_scores, Some(vec![-1.0, -2.0, -3.0]));
    }

    #[test]
    fn grid_search_preserves_input_order_for_equal_score_ties() {
        let combos = vec![params(5, 20), params(6, 24), params(8, 30)];
        let engine = StubEngine {
            metrics: vec![
                metrics_with_sharpe(1.0),
                metrics_with_sharpe(1.0),
                metrics_with_sharpe(0.5),
            ],
        };

        let result = grid_search(&engine, &combos, ObjectiveKind::Sharpe, 512, 3, false)
            .expect("grid result");

        assert_eq!(result.best_params.fast_len, 5);
        assert_eq!(result.best_params.slow_len, 20);
        assert_eq!(
            result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            vec![(5, 20), (6, 24), (8, 30)]
        );
    }

    #[test]
    fn grid_search_top_k_equal_scores_preserve_input_order_under_cap() {
        let combos = vec![params(5, 20), params(6, 24), params(8, 30), params(10, 40)];
        let engine = StubEngine {
            metrics: vec![
                metrics_with_sharpe(1.0),
                metrics_with_sharpe(1.0),
                metrics_with_sharpe(1.0),
                metrics_with_sharpe(0.5),
            ],
        };

        let result = grid_search(&engine, &combos, ObjectiveKind::Sharpe, 512, 2, false)
            .expect("grid result");

        assert_eq!(
            result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            vec![(5, 20), (6, 24)]
        );
        assert_eq!(result.top_scores, Some(vec![1.0, 1.0]));
    }

    #[test]
    fn grid_search_all_non_finite_scores_finalize_predictably() {
        let combos = vec![params(5, 20), params(6, 24)];
        let engine = StubEngine {
            metrics: vec![
                metrics_with_sharpe(f64::NAN),
                metrics_with_sharpe(f64::INFINITY),
            ],
        };

        let result = grid_search(&engine, &combos, ObjectiveKind::Sharpe, 512, 2, false)
            .expect("grid result");

        assert_eq!(result.best_params.fast_len, 5);
        assert_eq!(result.best_score, Some(f64::NEG_INFINITY));
        assert_eq!(
            result
                .top
                .iter()
                .map(|(params, _)| params.fast_len)
                .collect::<Vec<_>>(),
            vec![5, 6]
        );
    }

    #[test]
    fn grid_search_top_k_zero_keeps_empty_top_and_none_all_scores() {
        let combos = vec![params(5, 20), params(6, 24)];
        let engine = StubEngine {
            metrics: vec![metrics_with_sharpe(0.8), metrics_with_sharpe(0.4)],
        };

        let result = grid_search(&engine, &combos, ObjectiveKind::Sharpe, 512, 0, true)
            .expect("grid result");

        assert!(result.top.is_empty());
        assert_eq!(result.top_scores, Some(Vec::new()));
        assert!(result.all.is_some());
        assert_eq!(result.all.as_ref().map(Vec::len), Some(2));
        assert_eq!(result.all_scores, None);
    }

    #[test]
    fn grid_search_rejects_eval_batch_cardinality_mismatch() {
        let combos = vec![params(5, 20), params(6, 24)];

        let under = StubEngine {
            metrics: vec![metrics_with_sharpe(1.0)],
        };
        assert!(grid_search(&under, &combos, ObjectiveKind::Sharpe, 512, 2, false).is_none());

        let over = StubEngine {
            metrics: vec![
                metrics_with_sharpe(1.0),
                metrics_with_sharpe(0.5),
                metrics_with_sharpe(0.2),
            ],
        };
        assert!(grid_search(&over, &combos, ObjectiveKind::Sharpe, 512, 2, false).is_none());
    }
}
