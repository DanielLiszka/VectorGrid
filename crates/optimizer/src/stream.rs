use crate::{
    heatmap_axis_bins_for_period_span, normalize_rank_score, ObjectiveKind, OptimizationHeatmap,
    OptimizationResult, HEATMAP_CELL_ORDER_SLOW_FAST_ROW_MAJOR,
};
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use ta_strategies::double_ma::{DoubleMaParams, Metrics};

fn score(metrics: &Metrics, objective: ObjectiveKind) -> f64 {
    let s = match objective {
        ObjectiveKind::Pnl => metrics.pnl,
        ObjectiveKind::Sharpe => metrics.sharpe,
        ObjectiveKind::Sortino => metrics.sortino,
        ObjectiveKind::Calmar => metrics.calmar,
        ObjectiveKind::Omega => metrics.omega,
        ObjectiveKind::Volatility => -metrics.volatility,
        ObjectiveKind::MaxDrawdown => -metrics.max_dd,
    };
    normalize_rank_score(s)
}

#[derive(Clone, Debug)]
struct ScoredCombo {
    score: f64,
    tie_index: usize,
    params: DoubleMaParams,
    metrics: Metrics,
}

impl ScoredCombo {
    fn with_tie_offset(mut self, offset: usize) -> Self {
        self.tie_index = self.tie_index.saturating_add(offset);
        self
    }
}

#[derive(Clone, Debug)]
struct HeatmapAgg {
    bins_fast: usize,
    bins_slow: usize,
    fast_min: u16,
    fast_max: u16,
    slow_min: u16,
    slow_max: u16,
    values: Vec<f64>,
    sum_values: Vec<f64>,
    count_values: Vec<u32>,
}

impl HeatmapAgg {
    fn new(
        bins_fast: usize,
        bins_slow: usize,
        fast_min: u16,
        fast_max: u16,
        slow_min: u16,
        slow_max: u16,
    ) -> Self {
        let bins_fast = heatmap_axis_bins_for_period_span(bins_fast, fast_min, fast_max).max(1);
        let bins_slow = heatmap_axis_bins_for_period_span(bins_slow, slow_min, slow_max).max(1);
        let len = bins_fast.saturating_mul(bins_slow);
        Self {
            bins_fast,
            bins_slow,
            fast_min,
            fast_max,
            slow_min,
            slow_max,
            values: vec![f64::NEG_INFINITY; len],
            sum_values: vec![0.0; len],
            count_values: vec![0; len],
        }
    }

    fn bin_period(v: u16, v_min: u16, v_max: u16, bins: usize) -> usize {
        if bins <= 1 {
            return 0;
        }
        let denom = v_max.saturating_sub(v_min) as usize;
        if denom == 0 {
            return 0;
        }
        let num = v.saturating_sub(v_min) as usize;
        (num.saturating_mul(bins - 1)) / denom
    }

    fn push(&mut self, fast_len: u16, slow_len: u16, score: f64) {
        if !score.is_finite() || self.values.is_empty() {
            return;
        }

        let f_bin = Self::bin_period(fast_len, self.fast_min, self.fast_max, self.bins_fast)
            .min(self.bins_fast - 1);
        let s_bin = Self::bin_period(slow_len, self.slow_min, self.slow_max, self.bins_slow)
            .min(self.bins_slow - 1);

        let idx = s_bin.saturating_mul(self.bins_fast).saturating_add(f_bin);
        if let Some(cell) = self.values.get_mut(idx) {
            if score > *cell {
                *cell = score;
            }
        }
        if let Some(sum) = self.sum_values.get_mut(idx) {
            *sum += score;
        }
        if let Some(count) = self.count_values.get_mut(idx) {
            *count = count.saturating_add(1);
        }
    }

    fn finalize(self) -> OptimizationHeatmap {
        let values: Vec<Option<f64>> = self
            .values
            .into_iter()
            .map(|v| {
                if v.is_finite() && v != f64::NEG_INFINITY {
                    Some(v)
                } else {
                    None
                }
            })
            .collect();
        let mean_values: Vec<Option<f64>> = self
            .sum_values
            .into_iter()
            .zip(self.count_values.iter().copied())
            .map(|(sum, count)| {
                if count > 0 {
                    Some(sum / count as f64)
                } else {
                    None
                }
            })
            .collect();
        OptimizationHeatmap {
            bins_fast: self.bins_fast,
            bins_slow: self.bins_slow,
            fast_min: self.fast_min,
            fast_max: self.fast_max,
            slow_min: self.slow_min,
            slow_max: self.slow_max,
            cell_order: Some(HEATMAP_CELL_ORDER_SLOW_FAST_ROW_MAJOR.to_string()),
            values,
            mean_values: Some(mean_values),
            sample_counts: Some(self.count_values),
        }
    }

    fn matches_layout(&self, other: &Self) -> bool {
        self.bins_fast == other.bins_fast
            && self.bins_slow == other.bins_slow
            && self.fast_min == other.fast_min
            && self.fast_max == other.fast_max
            && self.slow_min == other.slow_min
            && self.slow_max == other.slow_max
    }
}

impl PartialEq for ScoredCombo {
    fn eq(&self, other: &Self) -> bool {
        self.score.to_bits() == other.score.to_bits() && self.tie_index == other.tie_index
    }
}

impl Eq for ScoredCombo {}

impl PartialOrd for ScoredCombo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredCombo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score
            .total_cmp(&other.score)
            .then_with(|| other.tie_index.cmp(&self.tie_index))
    }
}

pub struct StreamAggregator {
    objective: ObjectiveKind,
    score_fn: Option<fn(&Metrics) -> f64>,
    pair_score_fn: Option<fn(&DoubleMaParams, &Metrics) -> f64>,
    top_k: usize,
    num_candles: usize,

    num_combos: usize,
    best: Option<ScoredCombo>,
    heap: BinaryHeap<Reverse<ScoredCombo>>,
    all: Option<Vec<(DoubleMaParams, Metrics)>>,
    heatmap: Option<HeatmapAgg>,
}

impl StreamAggregator {
    pub fn new(
        objective: ObjectiveKind,
        top_k: usize,
        include_all: bool,
        num_candles: usize,
    ) -> Self {
        Self {
            objective,
            score_fn: None,
            pair_score_fn: None,
            top_k,
            num_candles,
            num_combos: 0,
            best: None,
            heap: BinaryHeap::new(),
            all: if include_all { Some(Vec::new()) } else { None },
            heatmap: None,
        }
    }

    pub fn with_score_fn(mut self, score_fn: fn(&Metrics) -> f64) -> Self {
        self.score_fn = Some(score_fn);
        self.pair_score_fn = None;
        self
    }

    pub fn with_pair_score_fn(
        mut self,
        pair_score_fn: fn(&DoubleMaParams, &Metrics) -> f64,
    ) -> Self {
        self.pair_score_fn = Some(pair_score_fn);
        self.score_fn = None;
        self
    }

    #[inline]
    fn score_value(&self, params: &DoubleMaParams, metrics: &Metrics) -> f64 {
        let score = if let Some(pair_score_fn) = self.pair_score_fn {
            pair_score_fn(params, metrics)
        } else if let Some(score_fn) = self.score_fn {
            score_fn(metrics)
        } else {
            score(metrics, self.objective)
        };
        normalize_rank_score(score)
    }

    pub fn with_heatmap(
        mut self,
        bins_fast: usize,
        bins_slow: usize,
        fast_min: u16,
        fast_max: u16,
        slow_min: u16,
        slow_max: u16,
    ) -> Self {
        self.heatmap = Some(HeatmapAgg::new(
            bins_fast, bins_slow, fast_min, fast_max, slow_min, slow_max,
        ));
        self
    }

    pub fn push(&mut self, params: DoubleMaParams, metrics: Metrics) {
        let tie_index = self.num_combos;
        self.num_combos = self.num_combos.saturating_add(1);
        let s = self.score_value(&params, &metrics);

        if let Some(h) = self.heatmap.as_mut() {
            h.push(params.fast_len, params.slow_len, s);
        }

        if let Some(all) = self.all.as_mut() {
            all.push((params.clone(), metrics.clone()));
        }

        let candidate = ScoredCombo {
            score: s,
            tie_index,
            params,
            metrics,
        };

        if self.top_k == 0 {
            match self.best.as_ref() {
                None => {
                    self.best = Some(candidate);
                }
                Some(b) if candidate.cmp(b).is_gt() => self.best = Some(candidate),
                _ => {}
            }
            return;
        }

        match self.best.as_ref() {
            None => {
                self.best = Some(candidate.clone());
            }
            Some(b) if candidate.cmp(b).is_gt() => self.best = Some(candidate.clone()),
            _ => {}
        }

        let entry = candidate;
        if self.heap.len() < self.top_k {
            self.heap.push(Reverse(entry));
            return;
        }

        if let Some(min_top) = self.heap.peek() {
            if entry.cmp(&min_top.0).is_gt() {
                let _ = self.heap.pop();
                self.heap.push(Reverse(entry));
            }
        }
    }

    pub fn num_combos(&self) -> usize {
        self.num_combos
    }

    pub fn merge(&mut self, other: StreamAggregator) {
        if self.objective != other.objective || self.num_candles != other.num_candles {
            return;
        }

        let self_score_fn = self.score_fn.map(|f| f as usize);
        let other_score_fn = other.score_fn.map(|f| f as usize);
        if self_score_fn != other_score_fn {
            return;
        }
        let self_pair_score_fn = self.pair_score_fn.map(|f| f as usize);
        let other_pair_score_fn = other.pair_score_fn.map(|f| f as usize);
        if self_pair_score_fn != other_pair_score_fn {
            return;
        }

        if self.top_k != other.top_k {
            return;
        }

        let self_had_combos = self.num_combos;
        let other_had_combos = other.num_combos;
        self.num_combos = self.num_combos.saturating_add(other.num_combos);

        let all_truthful = match (self.all.as_mut(), other.all) {
            (Some(a), Some(mut b)) => {
                a.append(&mut b);
                true
            }
            (Some(_), None) => other_had_combos == 0,
            (None, Some(b)) => {
                if self_had_combos == 0 {
                    self.all = Some(b);
                }
                true
            }
            (None, None) => true,
        };
        if !all_truthful {
            self.all = None;
        }

        match (self.best.take(), other.best) {
            (None, None) => {}
            (None, Some(b)) => self.best = Some(b.with_tie_offset(self_had_combos)),
            (Some(a), None) => self.best = Some(a),
            (Some(a), Some(b)) => {
                let b = b.with_tie_offset(self_had_combos);
                self.best = Some(if b.cmp(&a).is_gt() { b } else { a });
            }
        }

        if self.top_k > 0 {
            for rev in other.heap {
                let entry = rev.0.with_tie_offset(self_had_combos);
                if self.heap.len() < self.top_k {
                    self.heap.push(Reverse(entry));
                    continue;
                }
                if let Some(min_top) = self.heap.peek() {
                    if entry.cmp(&min_top.0).is_gt() {
                        let _ = self.heap.pop();
                        self.heap.push(Reverse(entry));
                    }
                }
            }
        }

        let heatmap_truthful = match (self.heatmap.as_mut(), other.heatmap) {
            (Some(a), Some(b)) => {
                if !a.matches_layout(&b) {
                    if self_had_combos == 0 {
                        self.heatmap = Some(b);
                        true
                    } else if other_had_combos == 0 {
                        true
                    } else {
                        false
                    }
                } else {
                    for (dst, src) in a.values.iter_mut().zip(b.values.into_iter()) {
                        if src > *dst {
                            *dst = src;
                        }
                    }
                    for (dst, src) in a.sum_values.iter_mut().zip(b.sum_values.into_iter()) {
                        *dst += src;
                    }
                    for (dst, src) in a.count_values.iter_mut().zip(b.count_values.into_iter()) {
                        *dst = dst.saturating_add(src);
                    }
                    true
                }
            }
            (None, Some(b)) => {
                if self_had_combos == 0 {
                    self.heatmap = Some(b);
                }
                true
            }
            (Some(_), None) => other_had_combos == 0,
            (None, None) => true,
        };
        if !heatmap_truthful {
            self.heatmap = None;
        }
    }

    pub fn finalize(self) -> Option<OptimizationResult> {
        let best = self.best?;

        let mut top: Vec<ScoredCombo> = self.heap.into_iter().map(|r| r.0).collect();
        top.sort_by(|a, b| b.cmp(a));
        let top_scores = top.iter().map(|x| x.score).collect();
        let top = top.into_iter().map(|x| (x.params, x.metrics)).collect();

        Some(OptimizationResult {
            best_params: best.params,
            best_metrics: best.metrics,
            best_score: Some(best.score),
            top,
            top_scores: Some(top_scores),
            all: self.all,
            all_scores: None,
            heatmap: self.heatmap.map(|h| h.finalize()),
            num_combos: self.num_combos,
            num_candles: self.num_candles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_with_sharpe(sharpe: f64) -> Metrics {
        Metrics {
            sharpe,
            ..Metrics::default()
        }
    }

    fn metrics_with_sharpe_and_pnl(sharpe: f64, pnl: f64) -> Metrics {
        Metrics {
            sharpe,
            pnl,
            ..Metrics::default()
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

    fn score_by_pnl(metrics: &Metrics) -> f64 {
        metrics.pnl
    }

    fn score_by_fast_len_or_nan(params: &DoubleMaParams, _metrics: &Metrics) -> f64 {
        if params.fast_len == 5 {
            f64::NAN
        } else {
            params.fast_len as f64
        }
    }

    #[test]
    fn heatmap_finalize_includes_mean_and_counts() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100)
            .with_heatmap(2, 2, 5, 15, 20, 30);
        agg.push(params(5, 20), metrics_with_sharpe(1.0));
        agg.push(params(5, 20), metrics_with_sharpe(3.0));

        let heatmap = agg.finalize().expect("result").heatmap.expect("heatmap");
        assert_eq!(heatmap.values[0], Some(3.0));
        assert_eq!(heatmap.mean_values.as_ref().expect("means")[0], Some(2.0));
        assert_eq!(heatmap.sample_counts.as_ref().expect("counts")[0], 2);
    }

    #[test]
    fn heatmap_merge_preserves_mean_and_counts() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100)
            .with_heatmap(2, 2, 5, 15, 20, 30);
        left.push(params(5, 20), metrics_with_sharpe(1.0));

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100)
            .with_heatmap(2, 2, 5, 15, 20, 30);
        right.push(params(5, 20), metrics_with_sharpe(3.0));

        left.merge(right);
        let heatmap = left.finalize().expect("result").heatmap.expect("heatmap");
        assert_eq!(heatmap.values[0], Some(3.0));
        assert_eq!(heatmap.mean_values.as_ref().expect("means")[0], Some(2.0));
        assert_eq!(heatmap.sample_counts.as_ref().expect("counts")[0], 2);
    }

    #[test]
    fn heatmap_uses_slow_fast_row_major_order_for_rectangular_surfaces() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100)
            .with_heatmap(3, 2, 5, 15, 20, 30);
        agg.push(params(15, 20), metrics_with_sharpe(1.25));

        let heatmap = agg.finalize().expect("result").heatmap.expect("heatmap");
        assert_eq!(heatmap.bins_fast, 3);
        assert_eq!(heatmap.bins_slow, 2);
        assert_eq!(
            heatmap.cell_order.as_deref(),
            Some(HEATMAP_CELL_ORDER_SLOW_FAST_ROW_MAJOR)
        );
        assert_eq!(heatmap.values[2], Some(1.25));
        assert_eq!(heatmap.sample_counts.as_ref().expect("counts")[2], 1);
    }

    #[test]
    fn heatmap_caps_bins_to_discrete_period_span() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100)
            .with_heatmap(128, 128, 5, 20, 20, 80);
        agg.push(params(5, 20), metrics_with_sharpe(1.0));

        let heatmap = agg.finalize().expect("result").heatmap.expect("heatmap");
        assert_eq!(heatmap.bins_fast, 16);
        assert_eq!(heatmap.bins_slow, 61);
        assert_eq!(heatmap.values.len(), 16 * 61);
        assert_eq!(heatmap.values[0], Some(1.0));
    }

    #[test]
    fn with_score_fn_uses_custom_ranking_instead_of_objective() {
        let mut agg =
            StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100).with_score_fn(score_by_pnl);
        agg.push(params(5, 20), metrics_with_sharpe_and_pnl(2.0, 10.0));
        agg.push(params(6, 24), metrics_with_sharpe_and_pnl(1.0, 25.0));

        let result = agg.finalize().expect("result");
        assert_eq!(result.best_params.fast_len, 6);
        assert_eq!(result.best_score, Some(25.0));
        assert_eq!(result.top_scores, Some(vec![25.0, 10.0]));
    }

    #[test]
    fn with_pair_score_fn_clamps_non_finite_values_and_keeps_finite_best() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100)
            .with_pair_score_fn(score_by_fast_len_or_nan);
        agg.push(params(5, 20), metrics_with_sharpe(10.0));
        agg.push(params(8, 30), metrics_with_sharpe(1.0));

        let result = agg.finalize().expect("result");
        assert_eq!(result.best_params.fast_len, 8);
        assert_eq!(result.best_score, Some(8.0));
        assert_eq!(result.top_scores, Some(vec![8.0, f64::NEG_INFINITY]));
    }

    #[test]
    fn equal_score_best_preserves_first_seen() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100);
        agg.push(params(5, 20), metrics_with_sharpe(1.0));
        agg.push(params(6, 24), metrics_with_sharpe(1.0));
        agg.push(params(8, 30), metrics_with_sharpe(1.0));

        let result = agg.finalize().expect("result");
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
    fn equal_score_topk_preserves_first_seen_under_cap() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100);
        agg.push(params(5, 20), metrics_with_sharpe(1.0));
        agg.push(params(6, 24), metrics_with_sharpe(1.0));
        agg.push(params(8, 30), metrics_with_sharpe(1.0));
        agg.push(params(10, 40), metrics_with_sharpe(0.5));

        let result = agg.finalize().expect("result");
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
    fn merge_equal_scores_is_left_before_right() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100);
        left.push(params(5, 20), metrics_with_sharpe(1.0));
        left.push(params(6, 24), metrics_with_sharpe(0.9));

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 3, false, 100);
        right.push(params(8, 30), metrics_with_sharpe(1.0));
        right.push(params(10, 40), metrics_with_sharpe(0.9));

        left.merge(right);
        let result = left.finalize().expect("result");

        assert_eq!(result.best_params.fast_len, 5);
        assert_eq!(
            result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            vec![(5, 20), (8, 30), (6, 24)]
        );
    }

    #[test]
    fn all_non_finite_scores_still_finalize_predictably() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100);
        agg.push(params(5, 20), metrics_with_sharpe(f64::NAN));
        agg.push(params(6, 24), metrics_with_sharpe(f64::INFINITY));
        agg.push(params(8, 30), metrics_with_sharpe(f64::NEG_INFINITY));

        let result = agg.finalize().expect("result");
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
    fn include_all_preserves_push_order_while_topk_remains_score_sorted() {
        let mut agg = StreamAggregator::new(ObjectiveKind::Sharpe, 2, true, 100);
        agg.push(params(5, 20), metrics_with_sharpe(0.5));
        agg.push(params(6, 24), metrics_with_sharpe(1.5));
        agg.push(params(8, 30), metrics_with_sharpe(1.0));

        let result = agg.finalize().expect("result");
        let all = result.all.expect("include-all rows");
        assert_eq!(
            all.iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            vec![(5, 20), (6, 24), (8, 30)]
        );
        assert_eq!(
            result
                .top
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            vec![(6, 24), (8, 30)]
        );
        assert_eq!(result.top_scores, Some(vec![1.5, 1.0]));
    }

    #[test]
    fn merge_invalidates_all_when_nonempty_side_lacks_full_detail() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 2, true, 100);
        left.push(params(5, 20), metrics_with_sharpe(1.0));

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100);
        right.push(params(8, 30), metrics_with_sharpe(3.0));

        left.merge(right);
        let result = left.finalize().expect("result");
        assert_eq!(result.best_params.fast_len, 8);
        assert_eq!(result.num_combos, 2);
        assert!(result.all.is_none());
    }

    #[test]
    fn merge_keeps_all_none_when_receiver_has_data_but_other_side_has_detail() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100);
        left.push(params(5, 20), metrics_with_sharpe(1.0));

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 2, true, 100);
        right.push(params(8, 30), metrics_with_sharpe(3.0));

        left.merge(right);
        let result = left.finalize().expect("result");
        assert_eq!(result.best_params.fast_len, 8);
        assert_eq!(result.num_combos, 2);
        assert!(result.all.is_none());
    }

    #[test]
    fn merge_adopts_all_from_nonempty_other_when_receiver_is_empty() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100);

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 2, true, 100);
        right.push(params(8, 30), metrics_with_sharpe(3.0));

        left.merge(right);
        let result = left.finalize().expect("result");
        assert_eq!(
            result
                .all
                .expect("empty receiver can adopt truthful all rows")
                .iter()
                .map(|(params, _)| (params.fast_len, params.slow_len))
                .collect::<Vec<_>>(),
            vec![(8, 30)]
        );
    }

    #[test]
    fn merge_invalidates_heatmap_when_nonempty_inputs_use_mismatched_layouts() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100)
            .with_heatmap(2, 2, 5, 15, 20, 30);
        left.push(params(5, 20), metrics_with_sharpe(1.0));

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100)
            .with_heatmap(3, 2, 5, 15, 20, 30);
        right.push(params(8, 30), metrics_with_sharpe(3.0));

        left.merge(right);
        let result = left.finalize().expect("result");
        assert_eq!(result.best_params.fast_len, 8);
        assert_eq!(result.num_combos, 2);
        assert!(result.heatmap.is_none());
    }

    #[test]
    fn merge_adopts_heatmap_from_nonempty_other_when_receiver_is_empty() {
        let mut left = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100);

        let mut right = StreamAggregator::new(ObjectiveKind::Sharpe, 2, false, 100)
            .with_heatmap(2, 2, 5, 15, 20, 30);
        right.push(params(8, 30), metrics_with_sharpe(3.0));

        left.merge(right);
        let result = left.finalize().expect("result");
        let heatmap = result
            .heatmap
            .expect("empty receiver can adopt truthful heatmap");
        assert_eq!(heatmap.values[2], Some(3.0));
        assert_eq!(heatmap.sample_counts.as_ref().expect("counts")[2], 1);
    }
}
