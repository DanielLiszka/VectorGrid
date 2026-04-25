use crate::options_types::{
    OptionsEvaluationSplit, OptionsSignalEvent, OptionsSignalEventManifestV1, SignalSide,
    OPTIONS_SIGNAL_MANIFEST_VERSION,
};
use crate::DoubleMaCurves;
use chrono::{DateTime, Utc};
use vector_ta::utilities::data_loader::Candles;

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

fn session_id_from_timestamp(timestamp_ms: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(timestamp_ms)
        .map(|value| value.format("%Y-%m-%d").to_string())
}

fn validate_entry_index(entry_idx: usize, candle_len: usize) -> Result<usize, String> {
    if entry_idx >= candle_len {
        return Err(format!(
            "trade entry index {entry_idx} out of bounds for candle length {candle_len}"
        ));
    }
    Ok(entry_idx)
}

fn resolve_closed_trade_exit(
    entry_idx: usize,
    exit_idx: usize,
    candle_len: usize,
    timestamps: &[i64],
) -> Result<(Option<usize>, Option<i64>), String> {
    if exit_idx < entry_idx {
        return Err(format!(
            "closed trade exit index {exit_idx} precedes entry index {entry_idx}"
        ));
    }
    if exit_idx >= candle_len {
        return Err(format!(
            "closed trade exit index {exit_idx} out of bounds for candle length {candle_len}"
        ));
    }
    let exit_timestamp = timestamps
        .get(exit_idx)
        .copied()
        .ok_or_else(|| format!("closed trade missing exit timestamp at index {exit_idx}"))?;
    Ok((Some(exit_idx), Some(exit_timestamp)))
}

#[allow(clippy::too_many_arguments)]
fn build_options_signal_manifest_from_double_like_curves(
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
        let entry_idx = validate_entry_index(trade.entry_t, candle_len)?;
        let decision_idx = if trade_on_next_bar && entry_idx > 0 {
            entry_idx - 1
        } else {
            entry_idx
        };
        let (base_exit_bar_index, base_exit_timestamp) = if trade.open {
            (None, None)
        } else {
            resolve_closed_trade_exit(entry_idx, trade.exit_t, candle_len, &candles.timestamp)?
        };

        let raw_underlier_price = candles
            .open
            .get(entry_idx)
            .copied()
            .or_else(|| candles.close.get(entry_idx).copied())
            .ok_or_else(|| format!("missing underlying price at entry index {entry_idx}"))?;
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

            session_id: session_id_from_timestamp(earliest_tradable_timestamp),
            regime_tag: None,
            confidence_tag: None,
            calendar_id: "market_hours_utc".to_string(),
        });
    }

    if events.is_empty() {
        return Err("signal-manifest projection produced no candidate signal events".to_string());
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
    build_options_signal_manifest_from_double_like_curves(
        parent_artifact_id,
        parent_run_kind,
        source_dataset_identity_hash,
        source_content_fingerprint,
        dataset_hash,
        timeframe,
        resample_spec_hash,
        adjustment_mode,
        symbol_root,
        trade_on_next_bar,
        candles,
        curves,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_options_signal_manifest_from_composite_trend(
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
    build_options_signal_manifest_from_double_like_curves(
        parent_artifact_id,
        parent_run_kind,
        source_dataset_identity_hash,
        source_content_fingerprint,
        dataset_hash,
        timeframe,
        resample_spec_hash,
        adjustment_mode,
        symbol_root,
        trade_on_next_bar,
        candles,
        curves,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_options_signal_manifest_from_regime_hybrid(
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
    build_options_signal_manifest_from_double_like_curves(
        parent_artifact_id,
        parent_run_kind,
        source_dataset_identity_hash,
        source_content_fingerprint,
        dataset_hash,
        timeframe,
        resample_spec_hash,
        adjustment_mode,
        symbol_root,
        trade_on_next_bar,
        candles,
        curves,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_options_signal_manifest_from_breakout_momentum(
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
    build_options_signal_manifest_from_double_like_curves(
        parent_artifact_id,
        parent_run_kind,
        source_dataset_identity_hash,
        source_content_fingerprint,
        dataset_hash,
        timeframe,
        resample_spec_hash,
        adjustment_mode,
        symbol_root,
        trade_on_next_bar,
        candles,
        curves,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ta_strategies::double_ma::{DoubleMaCurves, MinMaxBins, Trade};

    type ManifestBuilder = fn(
        &str,
        &str,
        &str,
        &str,
        &str,
        &str,
        Option<String>,
        &str,
        &str,
        bool,
        &Candles,
        &DoubleMaCurves,
    ) -> Result<OptionsSignalEventManifestV1, String>;

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

    fn make_curves(trades: Vec<Trade>) -> DoubleMaCurves {
        DoubleMaCurves {
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
            trades,
            trade_stats: None,
            indicator_bars: Vec::new(),
        }
    }

    fn build_manifest(
        builder: ManifestBuilder,
        candles: &Candles,
        curves: &DoubleMaCurves,
        trade_on_next_bar: bool,
    ) -> Result<OptionsSignalEventManifestV1, String> {
        builder(
            "artifact-a",
            "vectorgrid_run_artifact",
            "identity",
            "fingerprint",
            "dataset",
            "1m",
            None,
            "raw",
            "SPY",
            trade_on_next_bar,
            candles,
            curves,
        )
    }

    fn builder_cases() -> [(&'static str, ManifestBuilder); 4] {
        [
            ("double_ma", build_options_signal_manifest_from_double_ma),
            (
                "composite_trend",
                build_options_signal_manifest_from_composite_trend,
            ),
            (
                "regime_hybrid",
                build_options_signal_manifest_from_regime_hybrid,
            ),
            (
                "breakout_momentum",
                build_options_signal_manifest_from_breakout_momentum,
            ),
        ]
    }

    #[test]
    fn manifest_projection_uses_previous_bar_for_decision_when_trade_is_next_bar() {
        let candles = make_candles();
        let curves = make_curves(vec![Trade {
            entry_t: 1,
            exit_t: 2,
            direction: 1,
            pnl: 1.0,
            bars: 1,
            open: false,
        }]);
        let manifest = build_manifest(
            build_options_signal_manifest_from_double_ma,
            &candles,
            &curves,
            true,
        )
        .expect("manifest");
        let event = &manifest.events[0];
        assert_eq!(event.decision_bar_index, 0);
        assert_eq!(event.earliest_tradable_bar_index, 1);
        assert_eq!(event.side, SignalSide::Bullish);
        assert_eq!(event.raw_underlier_price, 101.0);
        assert_eq!(event.session_id.as_deref(), Some("2023-11-14"));
    }

    #[test]
    fn manifest_projection_uses_entry_bar_for_decision_when_trade_is_same_bar() {
        let candles = make_candles();
        let curves = make_curves(vec![Trade {
            entry_t: 1,
            exit_t: 2,
            direction: 1,
            pnl: 1.0,
            bars: 1,
            open: false,
        }]);

        let manifest = build_manifest(
            build_options_signal_manifest_from_double_ma,
            &candles,
            &curves,
            false,
        )
        .expect("manifest");
        let event = &manifest.events[0];
        assert_eq!(event.decision_bar_index, 1);
        assert_eq!(event.decision_timestamp, candles.timestamp[1]);
        assert_eq!(event.earliest_tradable_bar_index, 1);
    }

    #[test]
    fn manifest_projection_rejects_closed_trade_when_exit_precedes_entry() {
        let candles = make_candles();
        let curves = make_curves(vec![Trade {
            entry_t: 2,
            exit_t: 1,
            direction: 1,
            pnl: -1.0,
            bars: 1,
            open: false,
        }]);

        let err = build_manifest(
            build_options_signal_manifest_from_double_ma,
            &candles,
            &curves,
            true,
        )
        .expect_err("closed trade with reversed ordering should fail");
        assert!(err.contains("precedes entry index"));
    }

    #[test]
    fn manifest_projection_rejects_closed_trade_without_exit_timestamp() {
        let mut candles = make_candles();
        candles.timestamp.pop();
        let curves = make_curves(vec![Trade {
            entry_t: 1,
            exit_t: 2,
            direction: 1,
            pnl: -1.0,
            bars: 1,
            open: false,
        }]);

        let err = build_manifest(
            build_options_signal_manifest_from_double_ma,
            &candles,
            &curves,
            true,
        )
        .expect_err("closed trade with missing exit timestamp should fail");
        assert!(err.contains("missing exit timestamp"));
    }

    #[test]
    fn manifest_projection_leaves_open_trade_exit_metadata_absent() {
        let candles = make_candles();
        let curves = make_curves(vec![Trade {
            entry_t: 1,
            exit_t: usize::MAX,
            direction: 1,
            pnl: 0.0,
            bars: 999,
            open: true,
        }]);

        let manifest = build_manifest(
            build_options_signal_manifest_from_double_ma,
            &candles,
            &curves,
            true,
        )
        .expect("open trades should not require exit metadata");
        let event = &manifest.events[0];
        assert_eq!(event.base_exit_bar_index, None);
        assert_eq!(event.base_exit_timestamp, None);
    }

    #[test]
    fn manifest_projection_wrappers_share_same_projection_behavior() {
        let candles = make_candles();
        let curves = make_curves(vec![Trade {
            entry_t: 1,
            exit_t: 2,
            direction: 1,
            pnl: 1.0,
            bars: 1,
            open: false,
        }]);

        let baseline = build_manifest(
            build_options_signal_manifest_from_double_ma,
            &candles,
            &curves,
            true,
        )
        .expect("baseline manifest");
        let baseline_event = &baseline.events[0];

        for (label, builder) in builder_cases() {
            let manifest = build_manifest(builder, &candles, &curves, true)
                .unwrap_or_else(|err| panic!("builder '{label}' failed: {err}"));
            let event = &manifest.events[0];
            assert_eq!(manifest.events.len(), 1, "builder '{label}' event count");
            assert_eq!(
                manifest.version, baseline.version,
                "builder '{label}' version"
            );
            assert_eq!(
                manifest.calendar_id, baseline.calendar_id,
                "builder '{label}' calendar"
            );
            assert_eq!(
                event.decision_bar_index, baseline_event.decision_bar_index,
                "builder '{label}' decision index"
            );
            assert_eq!(
                event.earliest_tradable_bar_index, baseline_event.earliest_tradable_bar_index,
                "builder '{label}' entry index"
            );
            assert_eq!(
                event.base_exit_bar_index, baseline_event.base_exit_bar_index,
                "builder '{label}' exit index"
            );
            assert_eq!(
                event.base_exit_timestamp, baseline_event.base_exit_timestamp,
                "builder '{label}' exit timestamp"
            );
            assert_eq!(
                event.raw_underlier_price, baseline_event.raw_underlier_price,
                "builder '{label}' entry price"
            );
            assert_eq!(event.side, baseline_event.side, "builder '{label}' side");
            assert_eq!(
                event.session_id, baseline_event.session_id,
                "builder '{label}' session id"
            );
        }
    }

    #[test]
    fn manifest_projection_uses_strategy_neutral_no_signal_error() {
        let candles = make_candles();
        let curves = make_curves(vec![Trade {
            entry_t: 1,
            exit_t: 2,
            direction: 0,
            pnl: 0.0,
            bars: 1,
            open: false,
        }]);

        for (label, builder) in builder_cases() {
            let err = build_manifest(builder, &candles, &curves, true)
                .expect_err(&format!("builder '{label}' unexpectedly succeeded"));
            assert_eq!(
                err, "signal-manifest projection produced no candidate signal events",
                "builder '{label}' should use the shared no-signal error"
            );
        }
    }
}
