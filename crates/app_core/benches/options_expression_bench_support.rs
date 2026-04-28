use ta_app_core::options_types::OptionsStructureFamily;
use ta_app_core::{
    ImportedOptionsDataset, OptionType, OptionsContractSeries, OptionsDatasetManifestV1,
    OptionsEvaluationSplit, OptionsExitTemplateFamily, OptionsExpressionBackendPreference,
    OptionsExpressionRunRequest, OptionsFillModelFamily, OptionsMoneynessBucket, OptionsPriceRow,
    OptionsPriceSemantics, OptionsSignalEvent, OptionsSignalEventManifestV1,
    OptionsSplitAssignmentMode, OptionsStaticFilterBundle, SignalSide,
};

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;

fn minute_ts(base: i64, minute_index: usize) -> i64 {
    base + (minute_index as i64) * MINUTE_MS
}

fn make_underlier_path(rows: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(rows);
    let mut px = 100.0_f64;
    for idx in 0..rows {
        let drift = if idx < rows / 2 { 0.0008 } else { -0.0005 };
        let wave = ((idx as f64) * 0.083).sin() * 0.0020;
        px *= 1.0 + drift + wave;
        out.push(px.max(1.0));
    }
    out
}

fn option_mark_price(
    side: OptionType,
    strike: f64,
    underlying: f64,
    minute_index: usize,
    rows: usize,
) -> f64 {
    let intrinsic = match side {
        OptionType::Call => (underlying - strike).max(0.0),
        OptionType::Put => (strike - underlying).max(0.0),
    };
    let time_decay = 0.95 - 0.50 * (minute_index as f64 / rows.max(1) as f64);
    let wave = (((minute_index as f64) * 0.11) + strike * 0.02).sin().abs() * 0.06;
    (intrinsic * 0.58 + time_decay + wave).max(0.05)
}

fn make_contract_rows(
    base_ts: i64,
    underlying_path: &[f64],
    side: OptionType,
    strike: f64,
) -> Vec<OptionsPriceRow> {
    underlying_path
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, underlying)| {
            let close = option_mark_price(side, strike, underlying, idx, underlying_path.len());
            OptionsPriceRow {
                timestamp_ms: minute_ts(base_ts, idx),
                open: close * 0.98,
                high: close * 1.10,
                low: close * 0.90,
                close,
                underlying_price: underlying,
                bid: None,
                ask: None,
                mid: None,
                volume: Some(750.0 + (idx % 9) as f64 * 20.0),
                open_interest: Some(3_000.0),
                quote_age_ms: None,
                session_id: Some("bench_session".to_string()),
            }
        })
        .collect()
}

pub fn synthetic_dataset(rows: usize) -> ImportedOptionsDataset {
    let base_ts = 1_700_000_000_000_i64;
    let underlying_path = make_underlier_path(rows.max(120));
    let expiration_short = base_ts + 3 * DAY_MS;
    let expiration_long = base_ts + 5 * DAY_MS;
    let mut contracts = Vec::new();
    for (contract_id, side, strike, expiration_ts, expiration_date) in [
        (
            "SPY_C_ATM_3D",
            OptionType::Call,
            100.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_C_OTM_3D",
            OptionType::Call,
            102.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_C_ITM_5D",
            OptionType::Call,
            98.0,
            expiration_long,
            "2026-04-10",
        ),
        (
            "SPY_P_ATM_3D",
            OptionType::Put,
            100.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_P_OTM_3D",
            OptionType::Put,
            98.0,
            expiration_short,
            "2026-04-08",
        ),
        (
            "SPY_P_ITM_5D",
            OptionType::Put,
            102.0,
            expiration_long,
            "2026-04-10",
        ),
    ] {
        contracts.push(OptionsContractSeries {
            root_symbol: "SPY".to_string(),
            contract_id: contract_id.to_string(),
            option_type: side,
            expiration_date: expiration_date.to_string(),
            expiration_timestamp_ms: expiration_ts,
            strike,
            multiplier: 100.0,
            rows: make_contract_rows(base_ts, &underlying_path, side, strike),
        });
    }

    ImportedOptionsDataset {
        manifest: OptionsDatasetManifestV1 {
            version: 1,
            options_dataset_id: "bench_spy".to_string(),
            root_symbol: "SPY".to_string(),
            provider_name: "synthetic".to_string(),
            provider_source_surface: "minute_aggregate".to_string(),
            provider_entitlement_profile: None,
            quote_resolution: "1m".to_string(),
            price_semantics: OptionsPriceSemantics::MinuteAggregate,
            calendar_id: "market_hours_utc".to_string(),
            timezone: "UTC".to_string(),
            coverage_start: base_ts,
            coverage_end: minute_ts(base_ts, underlying_path.len().saturating_sub(1)),
            normalization_version: 1,
            quote_semantics_version: 1,
            corporate_action_mapping_version: 1,
            contract_reference_source: "synthetic".to_string(),
            contract_reference_as_of_utc: None,
            underlier_reference_source: "synthetic_underlier".to_string(),
            dataset_path: "synthetic".to_string(),
            row_count: contracts.iter().map(|series| series.rows.len()).sum(),
            contract_count: contracts.len(),
            aggregate_price_field: Some("close".to_string()),
            normalized_rows_sha256: None,
            manifest_semantic_sha256: None,
        },
        contracts,
    }
}

pub fn synthetic_manifest(events: usize, rows: usize) -> OptionsSignalEventManifestV1 {
    let base_ts = 1_700_000_000_000_i64;
    let underlying_path = make_underlier_path(rows.max(120));
    let event_count = events.max(12);
    let max_index = underlying_path.len().saturating_sub(10);
    let stride = (max_index / event_count.max(1)).max(4);
    let mut out = Vec::with_capacity(event_count);
    for idx in 0..event_count {
        let row_index = (8 + idx * stride).min(max_index);
        out.push(OptionsSignalEvent {
            event_id: format!("bench_evt_{idx:03}"),
            parent_artifact_id: "bench_parent".to_string(),
            parent_run_kind: "vectorgrid_run_artifact".to_string(),
            source_dataset_identity_hash: "bench_source".to_string(),
            source_content_fingerprint: "bench_fp".to_string(),
            dataset_hash: "bench_dataset_hash".to_string(),
            timeframe: "1m".to_string(),
            resample_spec_hash: None,
            adjustment_mode: "raw".to_string(),
            symbol_root: "SPY".to_string(),
            side: if idx % 2 == 0 {
                SignalSide::Bullish
            } else {
                SignalSide::Bearish
            },
            decision_timestamp: minute_ts(base_ts, row_index.saturating_sub(1)),
            earliest_tradable_timestamp: minute_ts(base_ts, row_index),
            decision_bar_index: row_index.saturating_sub(1),
            earliest_tradable_bar_index: row_index,
            raw_underlier_price: underlying_path[row_index],
            adjusted_underlier_price: None,
            base_entry_timestamp: Some(minute_ts(base_ts, row_index)),
            base_exit_timestamp: Some(minute_ts(base_ts, (row_index + 15).min(max_index))),
            base_exit_bar_index: Some((row_index + 15).min(max_index)),
            fold: OptionsEvaluationSplit::Search,
            session_id: Some("bench_session".to_string()),
            regime_tag: Some(if idx % 3 == 0 { "trend" } else { "range" }.to_string()),
            confidence_tag: Some(if idx % 2 == 0 { "high" } else { "medium" }.to_string()),
            calendar_id: "market_hours_utc".to_string(),
        });
    }

    OptionsSignalEventManifestV1 {
        version: 1,
        manifest_id: "bench_manifest".to_string(),
        parent_artifact_id: "bench_parent".to_string(),
        parent_run_kind: "vectorgrid_run_artifact".to_string(),
        source_dataset_identity_hash: "bench_source".to_string(),
        source_content_fingerprint: "bench_fp".to_string(),
        dataset_hash: "bench_dataset_hash".to_string(),
        timeframe: "1m".to_string(),
        resample_spec_hash: None,
        adjustment_mode: "raw".to_string(),
        symbol_root: "SPY".to_string(),
        calendar_id: "market_hours_utc".to_string(),
        runtime_dataset_id: None,
        events: out,
    }
}

pub fn synthetic_request() -> OptionsExpressionRunRequest {
    OptionsExpressionRunRequest {
        parent_artifact_path: "synthetic://bench".to_string(),
        parent_candidate_index: None,
        options_dataset_id: "bench_spy".to_string(),
        structure_family: OptionsStructureFamily::LongSingle,
        backend_preference: OptionsExpressionBackendPreference::Auto,
        delay_minutes: vec![0, 1, 5],
        dte_buckets: vec![1, 3, 5],
        moneyness_buckets: vec![
            OptionsMoneynessBucket::Atm,
            OptionsMoneynessBucket::Otm1,
            OptionsMoneynessBucket::Itm1,
        ],
        hold_minutes: vec![5, 15, 30],
        target_return_buckets: vec![0.25, 0.5],
        stop_return_buckets: vec![0.25, 0.5],
        exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
        fill_model: OptionsFillModelFamily::AggregateResearch,
        filters: OptionsStaticFilterBundle {
            max_entry_lag_minutes: Some(5),
            max_premium: Some(500.0),
            min_premium: Some(10.0),
            min_volume: Some(100.0),
            min_open_interest: Some(500.0),
            max_quote_age_ms: None,
            max_spread_bps: None,
        },
        top_k: 10,
        min_valid_trades: Some(1),
        split_ratio_search: 0.6,
        split_ratio_holdout: 0.2,
        split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
        sealed_finalist_count: 3,
    }
}

pub fn print_bench_metadata(group_name: &str, details: &[(&str, String)]) {
    let mut fields = vec![
        "timing_boundary=Criterion".to_string(),
        "timing_boundary_note=options_expression_backend_only".to_string(),
        "timing_boundary_exclusions=tauri_command_latency|frontend_hydration|artifact_io"
            .to_string(),
        "build_profile=bench".to_string(),
    ];
    fields.extend(details.iter().map(|(key, value)| format!("{key}={value}")));
    println!("benchmark metadata {group_name}: {}", fields.join(" "));
}
