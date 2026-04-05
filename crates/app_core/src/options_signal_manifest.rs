use crate::options_types::{
    OptionsEvaluationSplit, OptionsSignalEvent, OptionsSignalEventManifestV1, SignalSide,
    OPTIONS_SIGNAL_MANIFEST_VERSION,
};
use crate::DoubleMaCurves;
use vector_ta::utilities::data_loader::Candles;

fn clamp_index(idx: usize, len: usize) -> usize {
    idx.min(len.saturating_sub(1))
}

fn infer_side(direction: i32) -> Option<SignalSide> {
    if direction > 0 {
        Some(SignalSide::Bullish)
    } else if direction < 0 {
        Some(SignalSide::Bearish)
    } else {
        None
    }
}

fn build_event_id(parent_artifact_id: &str, index: usize) -> String {
    format!("{parent_artifact_id}:event:{index}")
}

#[allow(clippy::too_many_arguments)]
pub fn build_options_signal_manifest_from_double_ma(
    parent_artifact_id: &str,
    parent_run_kind: &str,
    source_dataset_identity_hash: &str,
    source_content_fingerprint: &str,
    dataset_hash: &str,
    timeframe: &str,
    resample_spec_hash: Option<String>,
    adjustment_mode: &str,
    symbol_root: &str,
    trade_on_next_bar: bool,
    candles: &Candles,
    curves: &DoubleMaCurves,
) -> Result<OptionsSignalEventManifestV1, String> {
    let candle_len = candles.close.len();
    if candle_len == 0 {
        return Err("cannot build options signal manifest from empty candles".to_string());
    }
    let mut events: Vec<OptionsSignalEvent> = Vec::new();
    for (idx, trade) in curves.trades.iter().enumerate() {
        let Some(side) = infer_side(trade.direction) else {
            continue;
        };
        let entry_idx = clamp_index(trade.entry_t, candle_len);
        let decision_idx = if trade_on_next_bar && entry_idx > 0 {
            entry_idx - 1
        } else {
            entry_idx
        };
        let exit_idx = clamp_index(trade.exit_t, candle_len);
        let raw_underlier_price = candles
            .open
            .get(entry_idx)
            .copied()
            .or_else(|| candles.close.get(entry_idx).copied())
            .ok_or_else(|| format!("missing underlying price at entry index {entry_idx}"))?;

        let base_exit_timestamp = if trade.open {
            None
        } else {
            candles.timestamp.get(exit_idx).copied()
        };
        let base_exit_bar_index = if trade.open { None } else { Some(exit_idx) };
        let decision_timestamp = candles
            .timestamp
            .get(decision_idx)
            .copied()
            .ok_or_else(|| format!("missing decision timestamp at index {decision_idx}"))?;
        let earliest_tradable_timestamp = candles
            .timestamp
            .get(entry_idx)
            .copied()
            .ok_or_else(|| format!("missing entry timestamp at index {entry_idx}"))?;
        let base_entry_timestamp = Some(earliest_tradable_timestamp);

        events.push(OptionsSignalEvent {
            event_id: build_event_id(parent_artifact_id, idx),
            parent_artifact_id: parent_artifact_id.to_string(),
            parent_run_kind: parent_run_kind.to_string(),
            source_dataset_identity_hash: source_dataset_identity_hash.to_string(),
            source_content_fingerprint: source_content_fingerprint.to_string(),
            dataset_hash: dataset_hash.to_string(),
            timeframe: timeframe.to_string(),
            resample_spec_hash: resample_spec_hash.clone(),
            adjustment_mode: adjustment_mode.to_string(),
            symbol_root: symbol_root.to_string(),
            side,
            decision_timestamp,
            earliest_tradable_timestamp,
            decision_bar_index: decision_idx,
            earliest_tradable_bar_index: entry_idx,
            raw_underlier_price,
            adjusted_underlier_price: None,
            base_entry_timestamp,
            base_exit_timestamp,
            base_exit_bar_index,
            fold: OptionsEvaluationSplit::Search,
            session_id: None,
            regime_tag: None,
            confidence_tag: None,
            calendar_id: "market_hours_utc".to_string(),
        });
    }

    if events.is_empty() {
        return Err("double_ma drilldown produced no candidate signal events".to_string());
    }

    Ok(OptionsSignalEventManifestV1 {
        version: OPTIONS_SIGNAL_MANIFEST_VERSION,
        manifest_id: format!("{parent_artifact_id}::options_signal_manifest_v1"),
        parent_artifact_id: parent_artifact_id.to_string(),
        parent_run_kind: parent_run_kind.to_string(),
        source_dataset_identity_hash: source_dataset_identity_hash.to_string(),
        source_content_fingerprint: source_content_fingerprint.to_string(),
        dataset_hash: dataset_hash.to_string(),
        timeframe: timeframe.to_string(),
        resample_spec_hash,
        adjustment_mode: adjustment_mode.to_string(),
        symbol_root: symbol_root.to_string(),
        calendar_id: "market_hours_utc".to_string(),
        runtime_dataset_id: None,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ta_strategies::double_ma::{DoubleMaCurves, MinMaxBins, Trade};

    fn make_candles() -> Candles {
        Candles::new(
            vec![1_700_000_000_000, 1_700_000_060_000, 1_700_000_120_000],
            vec![100.0, 101.0, 102.0],
            vec![101.0, 102.0, 103.0],
            vec![99.0, 100.0, 101.0],
            vec![100.5, 101.5, 102.5],
            vec![1_000.0, 1_100.0, 1_200.0],
        )
    }

    #[test]
    fn manifest_projection_uses_previous_bar_for_decision_when_trade_is_next_bar() {
        let candles = make_candles();
        let curves = DoubleMaCurves {
            equity: MinMaxBins {
                start_t: 0,
                end_t: 0,
                bins: 0,
                minmax: Vec::new(),
            },
            drawdown: MinMaxBins {
                start_t: 0,
                end_t: 0,
                bins: 0,
                minmax: Vec::new(),
            },
            trades: vec![Trade {
                entry_t: 1,
                exit_t: 2,
                direction: 1,
                pnl: 1.0,
                bars: 1,
                open: false,
            }],
            trade_stats: None,
            indicator_bars: Vec::new(),
        };
        let manifest = build_options_signal_manifest_from_double_ma(
            "artifact-a",
            "vectorgrid_run_artifact",
            "identity",
            "fingerprint",
            "dataset",
            "1m",
            None,
            "raw",
            "SPY",
            true,
            &candles,
            &curves,
        )
        .expect("manifest");
        let event = &manifest.events[0];
        assert_eq!(event.decision_bar_index, 0);
        assert_eq!(event.earliest_tradable_bar_index, 1);
        assert_eq!(event.side, SignalSide::Bullish);
        assert_eq!(event.raw_underlier_price, 101.0);
    }
}
