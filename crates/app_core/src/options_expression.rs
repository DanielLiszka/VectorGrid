use crate::options_types::{
    ImportedOptionsDataset, OptionType, OptionsCandidateInvalidReason,
    OptionsCandidateInvalidReasonCount, OptionsContractSeries, OptionsEvaluationSplit,
    OptionsExitTemplateFamily, OptionsExpressionDiagnostics, OptionsExpressionEventDetailRow,
    OptionsExpressionEventStatus, OptionsExpressionExitReason,
    OptionsExpressionFeasibilityEventCoverage, OptionsExpressionFeasibilityReport,
    OptionsExpressionFeasibilityTimingStats, OptionsExpressionMetrics, OptionsExpressionRunRequest,
    OptionsExpressionRunResult, OptionsExpressionStagingStats,
    OptionsExpressionTupleInspectionResult, OptionsExpressionTupleResult, OptionsFillModelFamily,
    OptionsMoneynessBucket, OptionsPriceRow, OptionsPriceSemantics, OptionsSignalEvent,
    OptionsSignalEventManifestV1, OptionsTupleSpec, SignalSide,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time::Instant;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AggregateStressMode {
    Baseline,
    Conservative,
}

#[derive(Clone, Copy, Debug)]
struct SelectedCandidate {
    contract_index: usize,
    entry_index: usize,
}

#[derive(Clone, Debug)]
struct TradeOutcome {
    pnl_per_contract: f64,
    premium_paid: f64,
    return_on_premium: f64,
}

#[derive(Clone, Debug)]
struct DetailedTradeOutcome {
    entry_timestamp_ms: i64,
    entry_price: f64,
    entry_underlier_price: f64,
    exit_timestamp_ms: i64,
    exit_price: f64,
    premium_paid: f64,
    pnl_per_contract: f64,
    return_on_premium: f64,
    exit_reason: OptionsExpressionExitReason,
}

impl DetailedTradeOutcome {
    fn summary(&self) -> TradeOutcome {
        TradeOutcome {
            pnl_per_contract: self.pnl_per_contract,
            premium_paid: self.premium_paid,
            return_on_premium: self.return_on_premium,
        }
    }
}

struct IndexedOptionsDataset<'a> {
    dataset: &'a ImportedOptionsDataset,
    calls: Vec<usize>,
    puts: Vec<usize>,
}

impl<'a> IndexedOptionsDataset<'a> {
    fn from_dataset(dataset: &'a ImportedOptionsDataset, symbol_root: &str) -> Self {
        let mut calls = Vec::new();
        let mut puts = Vec::new();
        for (idx, contract) in dataset.contracts.iter().enumerate() {
            if !contract.root_symbol.eq_ignore_ascii_case(symbol_root) {
                continue;
            }
            match contract.option_type {
                OptionType::Call => calls.push(idx),
                OptionType::Put => puts.push(idx),
            }
        }
        Self {
            dataset,
            calls,
            puts,
        }
    }

    fn contracts_for_side(&self, side: SignalSide) -> &[usize] {
        match side {
            SignalSide::Bullish => &self.calls,
            SignalSide::Bearish => &self.puts,
        }
    }

    fn contract(&self, contract_index: usize) -> &'a OptionsContractSeries {
        &self.dataset.contracts[contract_index]
    }
}

fn validate_fill_model(
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
) -> Result<(), String> {
    match (price_semantics, fill_model) {
        (OptionsPriceSemantics::MinuteAggregate, OptionsFillModelFamily::AggregateResearch) => {
            Ok(())
        }
        (OptionsPriceSemantics::DerivedMidSnapshot, OptionsFillModelFamily::AggregateResearch) => {
            Ok(())
        }
        (OptionsPriceSemantics::TopOfBookQuote, _) => Ok(()),
        _ => Err(format!(
            "fill model '{fill_model:?}' is incompatible with price semantics '{price_semantics:?}'"
        )),
    }
}

fn assign_splits(
    manifest: &OptionsSignalEventManifestV1,
    search_ratio: f64,
    holdout_ratio: f64,
) -> Result<Vec<OptionsSignalEvent>, String> {
    if !(0.0..=1.0).contains(&search_ratio) || !(0.0..=1.0).contains(&holdout_ratio) {
        return Err("split ratios must be between 0 and 1".to_string());
    }
    if search_ratio + holdout_ratio >= 1.0 {
        return Err("split_ratio_search + split_ratio_holdout must be < 1.0".to_string());
    }

    let mut events = manifest.events.clone();
    events.sort_by(|a, b| {
        a.earliest_tradable_timestamp
            .cmp(&b.earliest_tradable_timestamp)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });
    let total = events.len();
    if total == 0 {
        return Ok(events);
    }
    if total < 3 {
        for event in &mut events {
            event.fold = OptionsEvaluationSplit::Search;
        }
        return Ok(events);
    }

    let mut search_end = ((total as f64) * search_ratio).floor() as usize;
    let mut holdout_len = ((total as f64) * holdout_ratio).floor() as usize;
    search_end = search_end.clamp(1, total.saturating_sub(2));
    holdout_len = holdout_len.clamp(1, total.saturating_sub(search_end + 1));
    let holdout_end = (search_end + holdout_len).min(total.saturating_sub(1));

    for (idx, event) in events.iter_mut().enumerate() {
        event.fold = if idx < search_end {
            OptionsEvaluationSplit::Search
        } else if idx < holdout_end {
            OptionsEvaluationSplit::Holdout
        } else {
            OptionsEvaluationSplit::SealedTest
        };
    }
    Ok(events)
}

fn lower_bound_timestamp(rows: &[OptionsPriceRow], timestamp_ms: i64) -> Option<usize> {
    let idx = rows.partition_point(|row| row.timestamp_ms < timestamp_ms);
    if idx < rows.len() {
        Some(idx)
    } else {
        None
    }
}

fn actual_dte_days(expiration_timestamp_ms: i64, entry_timestamp_ms: i64) -> Option<u16> {
    if expiration_timestamp_ms <= entry_timestamp_ms {
        return None;
    }
    let delta = expiration_timestamp_ms - entry_timestamp_ms;
    let days = ((delta + DAY_MS - 1) / DAY_MS).max(1);
    u16::try_from(days).ok()
}

fn moneyness_target(
    side: SignalSide,
    underlier_price: f64,
    bucket: OptionsMoneynessBucket,
) -> (f64, Option<Ordering>) {
    let offset = match bucket {
        OptionsMoneynessBucket::Atm => 0.0,
        OptionsMoneynessBucket::Otm1 => 0.01,
        OptionsMoneynessBucket::Otm2 => 0.02,
        OptionsMoneynessBucket::Itm1 => -0.01,
    };
    match side {
        SignalSide::Bullish => {
            let target = underlier_price * (1.0 + offset);
            let direction = if offset > 0.0 {
                Some(Ordering::Greater)
            } else if offset < 0.0 {
                Some(Ordering::Less)
            } else {
                None
            };
            (target, direction)
        }
        SignalSide::Bearish => {
            let target = underlier_price * (1.0 - offset);
            let direction = if offset > 0.0 {
                Some(Ordering::Less)
            } else if offset < 0.0 {
                Some(Ordering::Greater)
            } else {
                None
            };
            (target, direction)
        }
    }
}

fn strike_matches_direction(
    strike: f64,
    underlier_price: f64,
    direction: Option<Ordering>,
) -> bool {
    match direction {
        Some(Ordering::Greater) => strike >= underlier_price,
        Some(Ordering::Less) => strike <= underlier_price,
        _ => true,
    }
}

fn aggregate_entry_price(row: &OptionsPriceRow, stress_mode: AggregateStressMode) -> f64 {
    match stress_mode {
        AggregateStressMode::Baseline => row.close,
        AggregateStressMode::Conservative => row.high.max(row.close),
    }
}

fn aggregate_exit_price(row: &OptionsPriceRow, stress_mode: AggregateStressMode) -> f64 {
    match stress_mode {
        AggregateStressMode::Baseline => row.close,
        AggregateStressMode::Conservative => row.low.min(row.close),
    }
}

fn quote_entry_price(row: &OptionsPriceRow, fill_model: OptionsFillModelFamily) -> Option<f64> {
    match fill_model {
        OptionsFillModelFamily::QuoteConservative => row.ask.or(row.mid).or(row.bid),
        OptionsFillModelFamily::QuoteMid => row
            .mid
            .or_else(|| row.ask.zip(row.bid).map(|(ask, bid)| (ask + bid) * 0.5))
            .or(row.bid)
            .or(row.ask),
        OptionsFillModelFamily::AggregateResearch => Some(row.close),
    }
}

fn quote_exit_price(row: &OptionsPriceRow, fill_model: OptionsFillModelFamily) -> Option<f64> {
    match fill_model {
        OptionsFillModelFamily::QuoteConservative => row.bid.or(row.mid).or(row.ask),
        OptionsFillModelFamily::QuoteMid => row
            .mid
            .or_else(|| row.ask.zip(row.bid).map(|(ask, bid)| (ask + bid) * 0.5))
            .or(row.bid)
            .or(row.ask),
        OptionsFillModelFamily::AggregateResearch => Some(row.close),
    }
}

fn option_entry_price_for_row(
    row: &OptionsPriceRow,
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
) -> Option<f64> {
    match price_semantics {
        OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
            Some(aggregate_entry_price(row, AggregateStressMode::Baseline))
        }
        OptionsPriceSemantics::TopOfBookQuote => quote_entry_price(row, fill_model),
    }
}

fn resolve_fixed_hold_cutoff(entry_timestamp_ms: i64, tuple: &OptionsTupleSpec) -> i64 {
    entry_timestamp_ms + i64::from(tuple.hold_minutes) * MINUTE_MS
}

fn resolve_eod_cutoff(contract_rows: &[OptionsPriceRow], entry_index: usize) -> i64 {
    let Some(entry_row) = contract_rows.get(entry_index) else {
        return i64::MAX;
    };
    let day_key = entry_row.timestamp_ms.div_euclid(DAY_MS);
    let mut cutoff = entry_row.timestamp_ms;
    for row in contract_rows.iter().skip(entry_index) {
        if row.timestamp_ms.div_euclid(DAY_MS) != day_key {
            break;
        }
        cutoff = row.timestamp_ms;
    }
    cutoff
}

fn find_cutoff_index(
    contract_rows: &[OptionsPriceRow],
    start_index: usize,
    cutoff_timestamp_ms: i64,
) -> Option<usize> {
    let lower = lower_bound_timestamp(&contract_rows[start_index..], cutoff_timestamp_ms)?;
    Some(start_index + lower)
}

fn select_candidate(
    event: &OptionsSignalEvent,
    tuple: &OptionsTupleSpec,
    index: &IndexedOptionsDataset<'_>,
    contracts: &[usize],
    price_semantics: OptionsPriceSemantics,
    filters: &crate::options_types::OptionsStaticFilterBundle,
    stress_mode: AggregateStressMode,
) -> Result<SelectedCandidate, OptionsCandidateInvalidReason> {
    if !matches!(
        (price_semantics, tuple.fill_model),
        (
            OptionsPriceSemantics::MinuteAggregate,
            OptionsFillModelFamily::AggregateResearch
        ) | (
            OptionsPriceSemantics::DerivedMidSnapshot,
            OptionsFillModelFamily::AggregateResearch
        ) | (OptionsPriceSemantics::TopOfBookQuote, _)
    ) {
        return Err(OptionsCandidateInvalidReason::IncompatiblePriceSemantics);
    }
    let requested_entry_ts =
        event.earliest_tradable_timestamp + i64::from(tuple.delay_minutes) * MINUTE_MS;
    let mut earliest_expiration: Option<i64> = None;
    let mut earliest_candidates: Vec<usize> = Vec::new();
    let mut saw_contract_as_of_event = false;
    for &contract_index in contracts {
        let contract = index.contract(contract_index);
        let Some(first_row) = contract.rows.first() else {
            continue;
        };
        if first_row.timestamp_ms > requested_entry_ts {
            continue;
        }
        saw_contract_as_of_event = true;
        let Some(dte_days) = actual_dte_days(contract.expiration_timestamp_ms, requested_entry_ts)
        else {
            continue;
        };
        if dte_days < tuple.dte_days {
            continue;
        }
        match earliest_expiration {
            Some(current) if contract.expiration_timestamp_ms > current => {}
            Some(current) if contract.expiration_timestamp_ms == current => {
                earliest_candidates.push(contract_index);
            }
            _ => {
                earliest_expiration = Some(contract.expiration_timestamp_ms);
                earliest_candidates.clear();
                earliest_candidates.push(contract_index);
            }
        }
    }
    if earliest_candidates.is_empty() {
        return Err(if saw_contract_as_of_event {
            OptionsCandidateInvalidReason::NoExpirationInBucket
        } else {
            OptionsCandidateInvalidReason::ContractNotEligibleAsOfEvent
        });
    }
    let (target_strike, direction) = moneyness_target(
        event.side,
        event.raw_underlier_price,
        tuple.moneyness_bucket,
    );

    let mut best_contract_index: Option<usize> = None;
    let mut best_distance = f64::INFINITY;
    for contract_index in earliest_candidates {
        let contract = index.contract(contract_index);
        if !strike_matches_direction(contract.strike, event.raw_underlier_price, direction) {
            continue;
        }
        let distance = (contract.strike - target_strike).abs();
        if distance < best_distance {
            best_distance = distance;
            best_contract_index = Some(contract_index);
        }
    }
    let Some(contract_index) = best_contract_index else {
        return Err(OptionsCandidateInvalidReason::NoStrikeInTolerance);
    };
    let contract = index.contract(contract_index);
    let Some(entry_index) = lower_bound_timestamp(&contract.rows, requested_entry_ts) else {
        return Err(OptionsCandidateInvalidReason::NoPriceRowAfterEntry);
    };
    let Some(entry_row) = contract.rows.get(entry_index) else {
        return Err(OptionsCandidateInvalidReason::NoPriceRowAfterEntry);
    };
    if let Some(max_lag_minutes) = filters.max_entry_lag_minutes {
        let max_lag_ms = i64::from(max_lag_minutes) * MINUTE_MS;
        if entry_row.timestamp_ms - requested_entry_ts > max_lag_ms {
            return Err(OptionsCandidateInvalidReason::EntryPastCutoff);
        }
    }
    let premium_paid = match tuple.fill_model {
        OptionsFillModelFamily::AggregateResearch => {
            aggregate_entry_price(entry_row, stress_mode) * contract.multiplier
        }
        OptionsFillModelFamily::QuoteConservative => {
            entry_row
                .ask
                .or(entry_row.mid)
                .or(entry_row.bid)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?
                * contract.multiplier
        }
        OptionsFillModelFamily::QuoteMid => {
            entry_row
                .mid
                .or_else(|| {
                    entry_row
                        .ask
                        .zip(entry_row.bid)
                        .map(|(ask, bid)| (ask + bid) * 0.5)
                })
                .or(entry_row.bid)
                .or(entry_row.ask)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?
                * contract.multiplier
        }
    };
    if let Some(min_premium) = filters.min_premium {
        if premium_paid < min_premium {
            return Err(OptionsCandidateInvalidReason::PremiumTooLow);
        }
    }
    if let Some(max_premium) = filters.max_premium {
        if premium_paid > max_premium {
            return Err(OptionsCandidateInvalidReason::PremiumTooHigh);
        }
    }
    if let Some(min_volume) = filters.min_volume {
        if entry_row.volume.unwrap_or_default() < min_volume {
            return Err(OptionsCandidateInvalidReason::InsufficientVolume);
        }
    }
    if let Some(min_open_interest) = filters.min_open_interest {
        if entry_row.open_interest.unwrap_or_default() < min_open_interest {
            return Err(OptionsCandidateInvalidReason::InsufficientOpenInterest);
        }
    }
    Ok(SelectedCandidate {
        contract_index,
        entry_index,
    })
}

fn cutoff_exit_reason(
    tuple: &OptionsTupleSpec,
    requested_cutoff: i64,
    forced_expiry_cutoff: i64,
) -> OptionsExpressionExitReason {
    if forced_expiry_cutoff < requested_cutoff {
        return OptionsExpressionExitReason::ForcedExpiryCutoff;
    }
    match tuple.exit_template {
        OptionsExitTemplateFamily::FixedHold | OptionsExitTemplateFamily::FixedHoldOrTargetStop => {
            OptionsExpressionExitReason::FixedHoldCutoff
        }
        OptionsExitTemplateFamily::EndOfDayOrTargetStop => {
            OptionsExpressionExitReason::EndOfDayCutoff
        }
        OptionsExitTemplateFamily::BaseExitOrTargetStop => {
            OptionsExpressionExitReason::BaseExitCutoff
        }
    }
}

fn evaluate_trade_detail(
    event: &OptionsSignalEvent,
    tuple: &OptionsTupleSpec,
    candidate: SelectedCandidate,
    index: &IndexedOptionsDataset<'_>,
    price_semantics: OptionsPriceSemantics,
    stress_mode: AggregateStressMode,
) -> Option<DetailedTradeOutcome> {
    let contract = index.contract(candidate.contract_index);
    let contract_rows = &contract.rows;
    let entry_row = contract_rows.get(candidate.entry_index)?;
    let entry_price = match price_semantics {
        OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
            aggregate_entry_price(entry_row, stress_mode)
        }
        OptionsPriceSemantics::TopOfBookQuote => quote_entry_price(entry_row, tuple.fill_model)?,
    };
    if !entry_price.is_finite() || entry_price <= 0.0 {
        return None;
    }
    let premium_paid = entry_price * contract.multiplier;
    let target_price = tuple
        .target_return
        .map(|pct| entry_price * (1.0 + pct.max(0.0)));
    let stop_price = tuple
        .stop_return
        .map(|pct| entry_price * (1.0 - pct.max(0.0)));
    let forced_expiry_cutoff = contract.expiration_timestamp_ms.saturating_sub(MINUTE_MS);
    let requested_cutoff = match tuple.exit_template {
        OptionsExitTemplateFamily::FixedHold => {
            resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)
        }
        OptionsExitTemplateFamily::FixedHoldOrTargetStop => {
            resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)
        }
        OptionsExitTemplateFamily::EndOfDayOrTargetStop => {
            resolve_eod_cutoff(contract_rows, candidate.entry_index)
        }
        OptionsExitTemplateFamily::BaseExitOrTargetStop => event
            .base_exit_timestamp
            .unwrap_or_else(|| resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)),
    };
    let cutoff_timestamp_ms = requested_cutoff.min(forced_expiry_cutoff);
    let cutoff_index =
        find_cutoff_index(contract_rows, candidate.entry_index, cutoff_timestamp_ms)?;

    for idx in (candidate.entry_index + 1)..=cutoff_index {
        let row = contract_rows.get(idx)?;
        match price_semantics {
            OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
                let target_hit = target_price
                    .map(|target| row.high >= target)
                    .unwrap_or(false);
                let stop_hit = stop_price.map(|stop| row.low <= stop).unwrap_or(false);
                if stop_hit {
                    let exit_price =
                        stop_price.unwrap_or_else(|| aggregate_exit_price(row, stress_mode));
                    let pnl_per_contract = (exit_price - entry_price) * contract.multiplier;
                    return Some(DetailedTradeOutcome {
                        entry_timestamp_ms: entry_row.timestamp_ms,
                        entry_price,
                        entry_underlier_price: entry_row.underlying_price,
                        exit_timestamp_ms: row.timestamp_ms,
                        exit_price,
                        pnl_per_contract,
                        premium_paid,
                        return_on_premium: pnl_per_contract / premium_paid,
                        exit_reason: OptionsExpressionExitReason::StopHit,
                    });
                }
                if target_hit {
                    let exit_price =
                        target_price.unwrap_or_else(|| aggregate_exit_price(row, stress_mode));
                    let pnl_per_contract = (exit_price - entry_price) * contract.multiplier;
                    return Some(DetailedTradeOutcome {
                        entry_timestamp_ms: entry_row.timestamp_ms,
                        entry_price,
                        entry_underlier_price: entry_row.underlying_price,
                        exit_timestamp_ms: row.timestamp_ms,
                        exit_price,
                        pnl_per_contract,
                        premium_paid,
                        return_on_premium: pnl_per_contract / premium_paid,
                        exit_reason: OptionsExpressionExitReason::TargetHit,
                    });
                }
            }
            OptionsPriceSemantics::TopOfBookQuote => {
                let exitable = quote_exit_price(row, tuple.fill_model)?;
                let target_hit = target_price
                    .map(|target| exitable >= target)
                    .unwrap_or(false);
                let stop_hit = stop_price.map(|stop| exitable <= stop).unwrap_or(false);
                if stop_hit {
                    let exit_price = stop_price.unwrap_or(exitable);
                    let pnl_per_contract = (exit_price - entry_price) * contract.multiplier;
                    return Some(DetailedTradeOutcome {
                        entry_timestamp_ms: entry_row.timestamp_ms,
                        entry_price,
                        entry_underlier_price: entry_row.underlying_price,
                        exit_timestamp_ms: row.timestamp_ms,
                        exit_price,
                        pnl_per_contract,
                        premium_paid,
                        return_on_premium: pnl_per_contract / premium_paid,
                        exit_reason: OptionsExpressionExitReason::StopHit,
                    });
                }
                if target_hit {
                    let exit_price = target_price.unwrap_or(exitable);
                    let pnl_per_contract = (exit_price - entry_price) * contract.multiplier;
                    return Some(DetailedTradeOutcome {
                        entry_timestamp_ms: entry_row.timestamp_ms,
                        entry_price,
                        entry_underlier_price: entry_row.underlying_price,
                        exit_timestamp_ms: row.timestamp_ms,
                        exit_price,
                        pnl_per_contract,
                        premium_paid,
                        return_on_premium: pnl_per_contract / premium_paid,
                        exit_reason: OptionsExpressionExitReason::TargetHit,
                    });
                }
            }
        }
    }

    let exit_row = contract_rows.get(cutoff_index)?;
    let exit_price = match price_semantics {
        OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
            aggregate_exit_price(exit_row, stress_mode)
        }
        OptionsPriceSemantics::TopOfBookQuote => quote_exit_price(exit_row, tuple.fill_model)?,
    };
    let pnl_per_contract = (exit_price - entry_price) * contract.multiplier;
    Some(DetailedTradeOutcome {
        entry_timestamp_ms: entry_row.timestamp_ms,
        entry_price,
        entry_underlier_price: entry_row.underlying_price,
        exit_timestamp_ms: exit_row.timestamp_ms,
        exit_price,
        pnl_per_contract,
        premium_paid,
        return_on_premium: pnl_per_contract / premium_paid,
        exit_reason: cutoff_exit_reason(tuple, requested_cutoff, forced_expiry_cutoff),
    })
}

fn median(mut values: Vec<f64>) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values[mid]
    } else {
        (values[mid - 1] + values[mid]) * 0.5
    }
}

fn compute_metrics(event_count: usize, outcomes: &[TradeOutcome]) -> OptionsExpressionMetrics {
    if event_count == 0 {
        return OptionsExpressionMetrics::default();
    }
    let valid_trades = outcomes.len();
    let invalid_events = event_count.saturating_sub(valid_trades);
    if valid_trades == 0 {
        return OptionsExpressionMetrics {
            valid_trades,
            invalid_events,
            invalid_rate: invalid_events as f64 / event_count as f64,
            expectancy_per_trade: 0.0,
            expectancy_per_premium: 0.0,
            median_return_on_premium: 0.0,
            win_rate: 0.0,
            total_pnl_per_contract: 0.0,
        };
    }

    let total_pnl: f64 = outcomes
        .iter()
        .map(|outcome| outcome.pnl_per_contract)
        .sum();
    let total_premium: f64 = outcomes.iter().map(|outcome| outcome.premium_paid).sum();
    let wins = outcomes
        .iter()
        .filter(|outcome| outcome.pnl_per_contract > 0.0)
        .count();
    let returns: Vec<f64> = outcomes
        .iter()
        .map(|outcome| outcome.return_on_premium)
        .collect();

    OptionsExpressionMetrics {
        valid_trades,
        invalid_events,
        invalid_rate: invalid_events as f64 / event_count as f64,
        expectancy_per_trade: total_pnl / valid_trades as f64,
        expectancy_per_premium: if total_premium > 0.0 {
            total_pnl / total_premium
        } else {
            0.0
        },
        median_return_on_premium: median(returns),
        win_rate: wins as f64 / valid_trades as f64,
        total_pnl_per_contract: total_pnl,
    }
}

#[derive(Clone, Debug)]
struct CandidateAxis {
    delay_minutes: Vec<u16>,
    dte_days: Vec<u16>,
    moneyness_buckets: Vec<OptionsMoneynessBucket>,
}

impl CandidateAxis {
    fn from_request(req: &OptionsExpressionRunRequest) -> Result<Self, String> {
        if req.delay_minutes.is_empty()
            || req.dte_buckets.is_empty()
            || req.moneyness_buckets.is_empty()
        {
            return Err(
                "delay_minutes, dte_buckets, and moneyness_buckets must not be empty".to_string(),
            );
        }
        Ok(Self {
            delay_minutes: req.delay_minutes.clone(),
            dte_days: req.dte_buckets.clone(),
            moneyness_buckets: req.moneyness_buckets.clone(),
        })
    }

    fn slot_count_per_event(&self) -> usize {
        self.delay_minutes.len() * self.dte_days.len() * self.moneyness_buckets.len()
    }

    fn slot_index(
        &self,
        event_index: usize,
        delay_index: usize,
        dte_index: usize,
        moneyness_index: usize,
    ) -> usize {
        let per_event = self.slot_count_per_event();
        let delay_stride = self.dte_days.len() * self.moneyness_buckets.len();
        let dte_stride = self.moneyness_buckets.len();
        event_index * per_event
            + delay_index * delay_stride
            + dte_index * dte_stride
            + moneyness_index
    }
}

#[derive(Clone, Debug)]
struct StructuralCandidateCache {
    axis: CandidateAxis,
    slots: Vec<Option<SelectedCandidate>>,
    invalid_reasons: Vec<Option<OptionsCandidateInvalidReason>>,
}

impl StructuralCandidateCache {
    fn candidate(
        &self,
        event_index: usize,
        delay_index: usize,
        dte_index: usize,
        moneyness_index: usize,
    ) -> Option<SelectedCandidate> {
        let slot_index = self
            .axis
            .slot_index(event_index, delay_index, dte_index, moneyness_index);
        self.slots.get(slot_index).copied().flatten()
    }

    fn candidate_slot_index(
        &self,
        event_index: usize,
        delay_index: usize,
        dte_index: usize,
        moneyness_index: usize,
    ) -> usize {
        self.axis
            .slot_index(event_index, delay_index, dte_index, moneyness_index)
    }

    fn invalid_reason_counts(&self) -> Vec<OptionsCandidateInvalidReasonCount> {
        let mut counts: BTreeMap<OptionsCandidateInvalidReason, usize> = BTreeMap::new();
        for reason in self.invalid_reasons.iter().flatten() {
            *counts.entry(*reason).or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(reason, count)| OptionsCandidateInvalidReasonCount { reason, count })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct ExitAxis {
    hold_minutes: Vec<u16>,
    target_values: Vec<Option<f64>>,
    stop_values: Vec<Option<f64>>,
    exit_template: OptionsExitTemplateFamily,
    fill_model: OptionsFillModelFamily,
}

impl ExitAxis {
    fn from_request(req: &OptionsExpressionRunRequest) -> Result<Self, String> {
        if req.hold_minutes.is_empty() {
            return Err("hold_minutes must not be empty".to_string());
        }
        let target_values = if matches!(req.exit_template, OptionsExitTemplateFamily::FixedHold) {
            vec![None]
        } else if req.target_return_buckets.is_empty() {
            vec![None]
        } else {
            req.target_return_buckets
                .iter()
                .copied()
                .map(Some)
                .collect()
        };
        let stop_values = if matches!(req.exit_template, OptionsExitTemplateFamily::FixedHold) {
            vec![None]
        } else if req.stop_return_buckets.is_empty() {
            vec![None]
        } else {
            req.stop_return_buckets.iter().copied().map(Some).collect()
        };
        Ok(Self {
            hold_minutes: req.hold_minutes.clone(),
            target_values,
            stop_values,
            exit_template: req.exit_template,
            fill_model: req.fill_model,
        })
    }

    fn summary_count_per_candidate(&self) -> usize {
        self.hold_minutes.len() * self.target_values.len() * self.stop_values.len()
    }

    fn summary_index(
        &self,
        candidate_slot_index: usize,
        hold_index: usize,
        target_index: usize,
        stop_index: usize,
    ) -> usize {
        let per_candidate = self.summary_count_per_candidate();
        let hold_stride = self.target_values.len() * self.stop_values.len();
        let target_stride = self.stop_values.len();
        candidate_slot_index * per_candidate
            + hold_index * hold_stride
            + target_index * target_stride
            + stop_index
    }
}

#[derive(Clone, Debug)]
struct ExitSummaryCache {
    axis: ExitAxis,
    outcomes: Vec<Option<TradeOutcome>>,
}

impl ExitSummaryCache {
    fn outcome(
        &self,
        candidate_slot_index: usize,
        hold_index: usize,
        target_index: usize,
        stop_index: usize,
    ) -> Option<&TradeOutcome> {
        let idx =
            self.axis
                .summary_index(candidate_slot_index, hold_index, target_index, stop_index);
        self.outcomes.get(idx).and_then(|entry| entry.as_ref())
    }
}

#[derive(Clone, Debug)]
struct CompiledTupleSpec {
    tuple: OptionsTupleSpec,
    delay_index: usize,
    dte_index: usize,
    moneyness_index: usize,
    hold_index: usize,
    target_index: usize,
    stop_index: usize,
}

fn compile_tuple_grid(
    candidate_axis: &CandidateAxis,
    exit_axis: &ExitAxis,
) -> Vec<CompiledTupleSpec> {
    let mut tuples = Vec::new();
    for (delay_index, delay_minutes) in candidate_axis.delay_minutes.iter().copied().enumerate() {
        for (dte_index, dte_days) in candidate_axis.dte_days.iter().copied().enumerate() {
            for (moneyness_index, moneyness_bucket) in
                candidate_axis.moneyness_buckets.iter().copied().enumerate()
            {
                for (hold_index, hold_minutes) in exit_axis.hold_minutes.iter().copied().enumerate()
                {
                    for (target_index, target_return) in
                        exit_axis.target_values.iter().copied().enumerate()
                    {
                        for (stop_index, stop_return) in
                            exit_axis.stop_values.iter().copied().enumerate()
                        {
                            tuples.push(CompiledTupleSpec {
                                tuple: OptionsTupleSpec {
                                    delay_minutes,
                                    dte_days,
                                    moneyness_bucket,
                                    hold_minutes,
                                    target_return,
                                    stop_return,
                                    exit_template: exit_axis.exit_template,
                                    fill_model: exit_axis.fill_model,
                                },
                                delay_index,
                                dte_index,
                                moneyness_index,
                                hold_index,
                                target_index,
                                stop_index,
                            });
                        }
                    }
                }
            }
        }
    }
    tuples
}

#[derive(Clone, Debug)]
struct RankedCompiledTuple {
    compiled: CompiledTupleSpec,
    result: OptionsExpressionTupleResult,
}

#[derive(Clone, Debug)]
struct PreparedOptionsExpressionRun {
    events: Vec<OptionsSignalEvent>,
    compiled_tuples: Vec<CompiledTupleSpec>,
    baseline_candidate_cache: StructuralCandidateCache,
    baseline_exit_summary_cache: ExitSummaryCache,
    conservative_candidate_cache: StructuralCandidateCache,
    conservative_exit_summary_cache: ExitSummaryCache,
    diagnostics: OptionsExpressionDiagnostics,
    staging: OptionsExpressionStagingStats,
    timings: OptionsExpressionFeasibilityTimingStats,
}

fn prepare_options_expression_run(
    req: &OptionsExpressionRunRequest,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<PreparedOptionsExpressionRun, String> {
    validate_fill_model(dataset.manifest.price_semantics, req.fill_model)?;
    if !dataset
        .manifest
        .root_symbol
        .eq_ignore_ascii_case(&signal_manifest.symbol_root)
    {
        return Err(format!(
            "signal root '{}' does not match options dataset root '{}'",
            signal_manifest.symbol_root, dataset.manifest.root_symbol
        ));
    }

    let events = assign_splits(
        signal_manifest,
        req.split_ratio_search,
        req.split_ratio_holdout,
    )?;
    let candidate_axis = CandidateAxis::from_request(req)?;
    let exit_axis = ExitAxis::from_request(req)?;
    let tuple_compile_start = Instant::now();
    let compiled_tuples = compile_tuple_grid(&candidate_axis, &exit_axis);
    let tuple_compile_ms = elapsed_ms(tuple_compile_start);
    let baseline_candidate_start = Instant::now();
    let baseline_candidate_cache = compile_structural_candidate_cache(
        &events,
        dataset,
        candidate_axis.clone(),
        req.fill_model,
        &req.filters,
        AggregateStressMode::Baseline,
    );
    let baseline_candidate_compile_ms = elapsed_ms(baseline_candidate_start);
    let baseline_exit_summary_start = Instant::now();
    let baseline_exit_summary_cache = compile_exit_summary_cache(
        &events,
        dataset,
        &baseline_candidate_cache,
        exit_axis.clone(),
        AggregateStressMode::Baseline,
    );
    let baseline_exit_summary_compile_ms = elapsed_ms(baseline_exit_summary_start);
    let conservative_candidate_start = Instant::now();
    let conservative_candidate_cache = compile_structural_candidate_cache(
        &events,
        dataset,
        candidate_axis,
        req.fill_model,
        &req.filters,
        AggregateStressMode::Conservative,
    );
    let conservative_candidate_compile_ms = elapsed_ms(conservative_candidate_start);
    let conservative_exit_summary_start = Instant::now();
    let conservative_exit_summary_cache = compile_exit_summary_cache(
        &events,
        dataset,
        &conservative_candidate_cache,
        exit_axis,
        AggregateStressMode::Conservative,
    );
    let conservative_exit_summary_compile_ms = elapsed_ms(conservative_exit_summary_start);
    let staging = OptionsExpressionStagingStats {
        candidate_slot_count: baseline_candidate_cache.slots.len(),
        compiled_candidate_count: baseline_candidate_cache
            .slots
            .iter()
            .filter(|entry| entry.is_some())
            .count(),
        baseline_exit_summary_count: baseline_exit_summary_cache.outcomes.len(),
        conservative_exit_summary_count: conservative_exit_summary_cache.outcomes.len(),
    };
    let total_events = events.len();
    let bullish_events = events
        .iter()
        .filter(|event| event.side == SignalSide::Bullish)
        .count();
    let bearish_events = total_events.saturating_sub(bullish_events);
    let search_events = events
        .iter()
        .filter(|event| event.fold == OptionsEvaluationSplit::Search)
        .count();
    let holdout_events = events
        .iter()
        .filter(|event| event.fold == OptionsEvaluationSplit::Holdout)
        .count();
    let sealed_test_events = events
        .iter()
        .filter(|event| event.fold == OptionsEvaluationSplit::SealedTest)
        .count();
    let diagnostics = OptionsExpressionDiagnostics {
        parent_artifact_path: req.parent_artifact_path.clone(),
        options_dataset_id: req.options_dataset_id.clone(),
        root_symbol: signal_manifest.symbol_root.clone(),
        total_events,
        bullish_events,
        bearish_events,
        search_events,
        holdout_events,
        sealed_test_events,
        dataset_contracts: dataset.manifest.contract_count,
        dataset_rows: dataset.manifest.row_count,
        coverage_start: Some(dataset.manifest.coverage_start),
        coverage_end: Some(dataset.manifest.coverage_end),
    };
    let timings = OptionsExpressionFeasibilityTimingStats {
        tuple_compile_ms,
        baseline_candidate_compile_ms,
        baseline_exit_summary_compile_ms,
        conservative_candidate_compile_ms,
        conservative_exit_summary_compile_ms,
        cpu_ranking_ms: 0,
    };

    Ok(PreparedOptionsExpressionRun {
        events,
        compiled_tuples,
        baseline_candidate_cache,
        baseline_exit_summary_cache,
        conservative_candidate_cache,
        conservative_exit_summary_cache,
        diagnostics,
        staging,
        timings,
    })
}

fn rank_compiled_tuples(
    req: &OptionsExpressionRunRequest,
    prepared: &PreparedOptionsExpressionRun,
) -> Vec<RankedCompiledTuple> {
    let holdout_events = prepared.diagnostics.holdout_events;
    let sealed_test_events = prepared.diagnostics.sealed_test_events;
    let mut ranked_results: Vec<RankedCompiledTuple> = Vec::new();
    for compiled_tuple in &prepared.compiled_tuples {
        let selection_metrics = evaluate_compiled_tuple_for_split(
            compiled_tuple,
            &prepared.events,
            OptionsEvaluationSplit::Search,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        );
        if let Some(min_valid_trades) = req.min_valid_trades {
            if selection_metrics.valid_trades < min_valid_trades {
                continue;
            }
        }
        let holdout_metrics = if holdout_events > 0 {
            Some(evaluate_compiled_tuple_for_split(
                compiled_tuple,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            ))
        } else {
            None
        };
        let sealed_metrics = if sealed_test_events > 0 {
            Some(evaluate_compiled_tuple_for_split(
                compiled_tuple,
                &prepared.events,
                OptionsEvaluationSplit::SealedTest,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            ))
        } else {
            None
        };
        let stress_metrics = if holdout_events > 0 {
            Some(evaluate_compiled_tuple_for_split(
                compiled_tuple,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.conservative_candidate_cache,
                &prepared.conservative_exit_summary_cache,
            ))
        } else {
            None
        };
        let robust = robust_score(
            &selection_metrics,
            holdout_metrics.as_ref(),
            sealed_metrics.as_ref(),
            stress_metrics.as_ref(),
        );

        ranked_results.push(RankedCompiledTuple {
            compiled: compiled_tuple.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled_tuple.tuple.clone(),
                selection_metrics,
                holdout_metrics,
                sealed_evaluation_metrics: sealed_metrics,
                stress_metrics,
                robust_score: robust,
            },
        });
    }

    ranked_results.sort_by(|a, b| {
        b.result
            .robust_score
            .partial_cmp(&a.result.robust_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                b.result
                    .selection_metrics
                    .expectancy_per_premium
                    .partial_cmp(&a.result.selection_metrics.expectancy_per_premium)
                    .unwrap_or(Ordering::Equal)
            })
    });
    ranked_results.truncate(req.top_k.max(1));
    ranked_results
}

fn compile_structural_candidate_cache(
    events: &[OptionsSignalEvent],
    dataset: &ImportedOptionsDataset,
    axis: CandidateAxis,
    fill_model: OptionsFillModelFamily,
    filters: &crate::options_types::OptionsStaticFilterBundle,
    stress_mode: AggregateStressMode,
) -> StructuralCandidateCache {
    let index = IndexedOptionsDataset::from_dataset(dataset, &dataset.manifest.root_symbol);
    let mut slots = vec![None; events.len() * axis.slot_count_per_event()];
    let mut invalid_reasons = vec![None; slots.len()];
    for (event_index, event) in events.iter().enumerate() {
        let contracts = index.contracts_for_side(event.side);
        for (delay_index, delay_minutes) in axis.delay_minutes.iter().copied().enumerate() {
            for (dte_index, dte_days) in axis.dte_days.iter().copied().enumerate() {
                for (moneyness_index, moneyness_bucket) in
                    axis.moneyness_buckets.iter().copied().enumerate()
                {
                    let slot_index =
                        axis.slot_index(event_index, delay_index, dte_index, moneyness_index);
                    let selector_tuple = OptionsTupleSpec {
                        delay_minutes,
                        dte_days,
                        moneyness_bucket,
                        hold_minutes: 1,
                        target_return: None,
                        stop_return: None,
                        exit_template: OptionsExitTemplateFamily::FixedHold,
                        fill_model,
                    };
                    match select_candidate(
                        event,
                        &selector_tuple,
                        &index,
                        contracts,
                        dataset.manifest.price_semantics,
                        filters,
                        stress_mode,
                    ) {
                        Ok(candidate) => slots[slot_index] = Some(candidate),
                        Err(reason) => invalid_reasons[slot_index] = Some(reason),
                    }
                }
            }
        }
    }
    StructuralCandidateCache {
        axis,
        slots,
        invalid_reasons,
    }
}

fn compile_exit_summary_cache(
    events: &[OptionsSignalEvent],
    dataset: &ImportedOptionsDataset,
    candidate_cache: &StructuralCandidateCache,
    axis: ExitAxis,
    stress_mode: AggregateStressMode,
) -> ExitSummaryCache {
    let index = IndexedOptionsDataset::from_dataset(dataset, &dataset.manifest.root_symbol);
    let outcome_count = candidate_cache.slots.len() * axis.summary_count_per_candidate();
    let mut outcomes = vec![None; outcome_count];
    for (event_index, event) in events.iter().enumerate() {
        for (delay_index, _) in candidate_cache.axis.delay_minutes.iter().enumerate() {
            for (dte_index, _) in candidate_cache.axis.dte_days.iter().enumerate() {
                for (moneyness_index, _) in
                    candidate_cache.axis.moneyness_buckets.iter().enumerate()
                {
                    let candidate_slot_index = candidate_cache.candidate_slot_index(
                        event_index,
                        delay_index,
                        dte_index,
                        moneyness_index,
                    );
                    let Some(candidate) = candidate_cache.candidate(
                        event_index,
                        delay_index,
                        dte_index,
                        moneyness_index,
                    ) else {
                        continue;
                    };
                    for (hold_index, hold_minutes) in axis.hold_minutes.iter().copied().enumerate()
                    {
                        for (target_index, target_return) in
                            axis.target_values.iter().copied().enumerate()
                        {
                            for (stop_index, stop_return) in
                                axis.stop_values.iter().copied().enumerate()
                            {
                                let summary_index = axis.summary_index(
                                    candidate_slot_index,
                                    hold_index,
                                    target_index,
                                    stop_index,
                                );
                                let eval_tuple = OptionsTupleSpec {
                                    delay_minutes: 0,
                                    dte_days: 0,
                                    moneyness_bucket: OptionsMoneynessBucket::Atm,
                                    hold_minutes,
                                    target_return,
                                    stop_return,
                                    exit_template: axis.exit_template,
                                    fill_model: axis.fill_model,
                                };
                                outcomes[summary_index] = evaluate_trade_detail(
                                    event,
                                    &eval_tuple,
                                    candidate,
                                    &index,
                                    dataset.manifest.price_semantics,
                                    stress_mode,
                                )
                                .map(|detail| detail.summary());
                            }
                        }
                    }
                }
            }
        }
    }
    ExitSummaryCache { axis, outcomes }
}

fn evaluate_compiled_tuple_for_split(
    tuple: &CompiledTupleSpec,
    events: &[OptionsSignalEvent],
    split: OptionsEvaluationSplit,
    candidate_cache: &StructuralCandidateCache,
    exit_summary_cache: &ExitSummaryCache,
) -> OptionsExpressionMetrics {
    let mut split_event_count = 0usize;
    let mut outcomes: Vec<TradeOutcome> = Vec::new();
    for (event_index, event) in events.iter().enumerate() {
        if event.fold != split {
            continue;
        }
        split_event_count += 1;
        let candidate_slot_index = candidate_cache.candidate_slot_index(
            event_index,
            tuple.delay_index,
            tuple.dte_index,
            tuple.moneyness_index,
        );
        if let Some(outcome) = exit_summary_cache
            .outcome(
                candidate_slot_index,
                tuple.hold_index,
                tuple.target_index,
                tuple.stop_index,
            )
            .cloned()
        {
            outcomes.push(outcome);
        }
    }
    compute_metrics(split_event_count, &outcomes)
}

fn robust_score(
    search: &OptionsExpressionMetrics,
    holdout: Option<&OptionsExpressionMetrics>,
    sealed: Option<&OptionsExpressionMetrics>,
    stress: Option<&OptionsExpressionMetrics>,
) -> f64 {
    let coverage = (1.0 - search.invalid_rate).clamp(0.0, 1.0);
    let search_score = search.expectancy_per_premium;
    let holdout_score = holdout
        .map(|metrics| metrics.expectancy_per_premium)
        .unwrap_or(search_score);
    let stress_score = stress
        .map(|metrics| metrics.expectancy_per_premium)
        .unwrap_or(search_score);
    let sealed_score = sealed
        .map(|metrics| metrics.expectancy_per_premium)
        .unwrap_or(holdout_score);
    coverage
        * (0.5 * search_score + 0.25 * holdout_score + 0.15 * stress_score + 0.10 * sealed_score)
}

fn build_event_detail_row(
    event_index: usize,
    event: &OptionsSignalEvent,
    compiled_tuple: &CompiledTupleSpec,
    index: &IndexedOptionsDataset<'_>,
    candidate_cache: &StructuralCandidateCache,
) -> OptionsExpressionEventDetailRow {
    let requested_entry_timestamp = event.earliest_tradable_timestamp
        + i64::from(compiled_tuple.tuple.delay_minutes) * MINUTE_MS;
    let candidate = candidate_cache.candidate(
        event_index,
        compiled_tuple.delay_index,
        compiled_tuple.dte_index,
        compiled_tuple.moneyness_index,
    );
    let Some(candidate) = candidate else {
        return OptionsExpressionEventDetailRow {
            event_index,
            event_id: event.event_id.clone(),
            split: event.fold,
            side: event.side,
            decision_timestamp: event.decision_timestamp,
            earliest_tradable_timestamp: event.earliest_tradable_timestamp,
            requested_entry_timestamp,
            raw_underlier_price: event.raw_underlier_price,
            base_exit_timestamp: event.base_exit_timestamp,
            status: OptionsExpressionEventStatus::CandidateMissing,
            contract_id: None,
            expiration_date: None,
            strike: None,
            multiplier: None,
            entry_timestamp: None,
            entry_option_price: None,
            entry_underlier_price: None,
            exit_timestamp: None,
            exit_option_price: None,
            premium_paid: None,
            pnl_per_contract: None,
            return_on_premium: None,
            exit_reason: None,
        };
    };

    let contract = index.contract(candidate.contract_index);
    let entry_row = contract.rows.get(candidate.entry_index);
    let fallback_entry_price = entry_row.and_then(|row| {
        option_entry_price_for_row(
            row,
            index.dataset.manifest.price_semantics,
            compiled_tuple.tuple.fill_model,
        )
    });
    let detail = evaluate_trade_detail(
        event,
        &compiled_tuple.tuple,
        candidate,
        index,
        index.dataset.manifest.price_semantics,
        AggregateStressMode::Baseline,
    );
    let status = if detail.is_some() {
        OptionsExpressionEventStatus::Executed
    } else {
        OptionsExpressionEventStatus::OutcomeMissing
    };

    OptionsExpressionEventDetailRow {
        event_index,
        event_id: event.event_id.clone(),
        split: event.fold,
        side: event.side,
        decision_timestamp: event.decision_timestamp,
        earliest_tradable_timestamp: event.earliest_tradable_timestamp,
        requested_entry_timestamp,
        raw_underlier_price: event.raw_underlier_price,
        base_exit_timestamp: event.base_exit_timestamp,
        status,
        contract_id: Some(contract.contract_id.clone()),
        expiration_date: Some(contract.expiration_date.clone()),
        strike: Some(contract.strike),
        multiplier: Some(contract.multiplier),
        entry_timestamp: detail
            .as_ref()
            .map(|value| value.entry_timestamp_ms)
            .or_else(|| entry_row.map(|row| row.timestamp_ms)),
        entry_option_price: detail
            .as_ref()
            .map(|value| value.entry_price)
            .or(fallback_entry_price),
        entry_underlier_price: detail
            .as_ref()
            .map(|value| value.entry_underlier_price)
            .or_else(|| entry_row.map(|row| row.underlying_price)),
        exit_timestamp: detail.as_ref().map(|value| value.exit_timestamp_ms),
        exit_option_price: detail.as_ref().map(|value| value.exit_price),
        premium_paid: detail.as_ref().map(|value| value.premium_paid),
        pnl_per_contract: detail.as_ref().map(|value| value.pnl_per_contract),
        return_on_premium: detail.as_ref().map(|value| value.return_on_premium),
        exit_reason: detail.as_ref().map(|value| value.exit_reason),
    }
}

fn event_has_any_candidate(candidate_cache: &StructuralCandidateCache, event_index: usize) -> bool {
    candidate_cache
        .axis
        .delay_minutes
        .iter()
        .enumerate()
        .any(|(delay_index, _)| {
            candidate_cache
                .axis
                .dte_days
                .iter()
                .enumerate()
                .any(|(dte_index, _)| {
                    candidate_cache
                        .axis
                        .moneyness_buckets
                        .iter()
                        .enumerate()
                        .any(|(moneyness_index, _)| {
                            candidate_cache
                                .candidate(event_index, delay_index, dte_index, moneyness_index)
                                .is_some()
                        })
                })
        })
}

fn event_has_any_outcome(
    candidate_cache: &StructuralCandidateCache,
    exit_summary_cache: &ExitSummaryCache,
    event_index: usize,
) -> bool {
    candidate_cache
        .axis
        .delay_minutes
        .iter()
        .enumerate()
        .any(|(delay_index, _)| {
            candidate_cache
                .axis
                .dte_days
                .iter()
                .enumerate()
                .any(|(dte_index, _)| {
                    candidate_cache
                        .axis
                        .moneyness_buckets
                        .iter()
                        .enumerate()
                        .any(|(moneyness_index, _)| {
                            let candidate_slot_index = candidate_cache.candidate_slot_index(
                                event_index,
                                delay_index,
                                dte_index,
                                moneyness_index,
                            );
                            candidate_cache
                                .candidate(event_index, delay_index, dte_index, moneyness_index)
                                .is_some()
                                && exit_summary_cache.axis.hold_minutes.iter().enumerate().any(
                                    |(hold_index, _)| {
                                        exit_summary_cache
                                            .axis
                                            .target_values
                                            .iter()
                                            .enumerate()
                                            .any(|(target_index, _)| {
                                                exit_summary_cache
                                                    .axis
                                                    .stop_values
                                                    .iter()
                                                    .enumerate()
                                                    .any(|(stop_index, _)| {
                                                        exit_summary_cache
                                                            .outcome(
                                                                candidate_slot_index,
                                                                hold_index,
                                                                target_index,
                                                                stop_index,
                                                            )
                                                            .is_some()
                                                    })
                                            })
                                    },
                                )
                        })
                })
        })
}

fn split_coverage_counts<F>(
    events: &[OptionsSignalEvent],
    mut predicate: F,
) -> (usize, usize, usize, usize)
where
    F: FnMut(usize) -> bool,
{
    let mut total = 0usize;
    let mut search = 0usize;
    let mut holdout = 0usize;
    let mut sealed = 0usize;
    for (event_index, event) in events.iter().enumerate() {
        if !predicate(event_index) {
            continue;
        }
        total += 1;
        match event.fold {
            OptionsEvaluationSplit::Search => search += 1,
            OptionsEvaluationSplit::Holdout => holdout += 1,
            OptionsEvaluationSplit::SealedTest => sealed += 1,
        }
    }
    (total, search, holdout, sealed)
}

pub fn run_options_expression_blocking(
    req: OptionsExpressionRunRequest,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionRunResult, String> {
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let ranked_results: Vec<OptionsExpressionTupleResult> = rank_compiled_tuples(&req, &prepared)
        .into_iter()
        .map(|entry| entry.result)
        .collect();

    Ok(OptionsExpressionRunResult {
        diagnostics: prepared.diagnostics,
        backend_used: "cpu_reference".to_string(),
        price_semantics: dataset.manifest.price_semantics,
        tuple_count: ranked_results.len(),
        staging: Some(prepared.staging),
        ranked_results,
    })
}

pub fn analyze_options_expression_feasibility_blocking(
    req: OptionsExpressionRunRequest,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionFeasibilityReport, String> {
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let rank_start = Instant::now();
    let _ = rank_compiled_tuples(&req, &prepared);
    let cpu_ranking_ms = elapsed_ms(rank_start);
    let candidate_slot_count = prepared.staging.candidate_slot_count;
    let compiled_candidate_count = prepared.staging.compiled_candidate_count;
    let invalid_slot_count = candidate_slot_count.saturating_sub(compiled_candidate_count);
    let baseline_exit_summary_count = prepared.staging.baseline_exit_summary_count;
    let conservative_exit_summary_count = prepared.staging.conservative_exit_summary_count;
    let baseline_compiled_exit_summary_count = prepared
        .baseline_exit_summary_cache
        .outcomes
        .iter()
        .filter(|entry| entry.is_some())
        .count();
    let conservative_compiled_exit_summary_count = prepared
        .conservative_exit_summary_cache
        .outcomes
        .iter()
        .filter(|entry| entry.is_some())
        .count();
    let (
        total_events_with_candidates,
        search_events_with_candidates,
        holdout_events_with_candidates,
        sealed_test_events_with_candidates,
    ) = split_coverage_counts(&prepared.events, |event_index| {
        event_has_any_candidate(&prepared.baseline_candidate_cache, event_index)
    });
    let (
        total_events_with_baseline_outcomes,
        search_events_with_baseline_outcomes,
        holdout_events_with_baseline_outcomes,
        sealed_test_events_with_baseline_outcomes,
    ) = split_coverage_counts(&prepared.events, |event_index| {
        event_has_any_outcome(
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
            event_index,
        )
    });
    let (
        total_events_with_conservative_outcomes,
        search_events_with_conservative_outcomes,
        holdout_events_with_conservative_outcomes,
        sealed_test_events_with_conservative_outcomes,
    ) = split_coverage_counts(&prepared.events, |event_index| {
        event_has_any_outcome(
            &prepared.conservative_candidate_cache,
            &prepared.conservative_exit_summary_cache,
            event_index,
        )
    });
    let mut timings = prepared.timings.clone();
    timings.cpu_ranking_ms = cpu_ranking_ms;
    let event_coverage = OptionsExpressionFeasibilityEventCoverage {
        total_events_with_candidates,
        search_events_with_candidates,
        holdout_events_with_candidates,
        sealed_test_events_with_candidates,
        total_events_with_baseline_outcomes,
        search_events_with_baseline_outcomes,
        holdout_events_with_baseline_outcomes,
        sealed_test_events_with_baseline_outcomes,
        total_events_with_conservative_outcomes,
        search_events_with_conservative_outcomes,
        holdout_events_with_conservative_outcomes,
        sealed_test_events_with_conservative_outcomes,
    };

    Ok(OptionsExpressionFeasibilityReport {
        diagnostics: prepared.diagnostics,
        price_semantics: dataset.manifest.price_semantics,
        fill_model: req.fill_model,
        compiled_tuple_count: prepared.compiled_tuples.len(),
        per_event_candidate_slots: prepared
            .baseline_candidate_cache
            .axis
            .slot_count_per_event(),
        per_candidate_exit_summaries: prepared
            .baseline_exit_summary_cache
            .axis
            .summary_count_per_candidate(),
        candidate_slot_count,
        compiled_candidate_count,
        candidate_density: ratio_or_zero(compiled_candidate_count, candidate_slot_count),
        invalid_slot_count,
        invalid_slot_rate: ratio_or_zero(invalid_slot_count, candidate_slot_count),
        baseline_exit_summary_count,
        baseline_compiled_exit_summary_count,
        baseline_exit_summary_density: ratio_or_zero(
            baseline_compiled_exit_summary_count,
            baseline_exit_summary_count,
        ),
        conservative_exit_summary_count,
        conservative_compiled_exit_summary_count,
        conservative_exit_summary_density: ratio_or_zero(
            conservative_compiled_exit_summary_count,
            conservative_exit_summary_count,
        ),
        projected_event_tuple_evaluations: (prepared.compiled_tuples.len() as u64)
            .saturating_mul(prepared.events.len() as u64),
        structural_cache_bytes_estimate: prepared.baseline_candidate_cache.slots.len()
            * std::mem::size_of::<Option<SelectedCandidate>>()
            + prepared.baseline_candidate_cache.invalid_reasons.len()
                * std::mem::size_of::<Option<OptionsCandidateInvalidReason>>(),
        baseline_summary_bytes_estimate: prepared.baseline_exit_summary_cache.outcomes.len()
            * std::mem::size_of::<Option<TradeOutcome>>(),
        conservative_summary_bytes_estimate: prepared
            .conservative_exit_summary_cache
            .outcomes
            .len()
            * std::mem::size_of::<Option<TradeOutcome>>(),
        timings,
        event_coverage,
        invalid_reasons: prepared.baseline_candidate_cache.invalid_reason_counts(),
    })
}

fn ratio_or_zero(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

pub fn inspect_options_expression_tuple_blocking(
    req: OptionsExpressionRunRequest,
    tuple_index: usize,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionTupleInspectionResult, String> {
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let ranked = rank_compiled_tuples(&req, &prepared);
    let tuple_count = ranked.len();
    let selected = ranked.get(tuple_index).ok_or_else(|| {
        format!(
            "tuple_index {} is out of range for {} ranked tuple{}",
            tuple_index,
            tuple_count,
            if tuple_count == 1 { "" } else { "s" }
        )
    })?;
    let index = IndexedOptionsDataset::from_dataset(dataset, &dataset.manifest.root_symbol);
    let rows: Vec<OptionsExpressionEventDetailRow> = prepared
        .events
        .iter()
        .enumerate()
        .map(|(event_index, event)| {
            build_event_detail_row(
                event_index,
                event,
                &selected.compiled,
                &index,
                &prepared.baseline_candidate_cache,
            )
        })
        .collect();
    let executed_events = rows
        .iter()
        .filter(|row| row.status == OptionsExpressionEventStatus::Executed)
        .count();
    let candidate_missing_events = rows
        .iter()
        .filter(|row| row.status == OptionsExpressionEventStatus::CandidateMissing)
        .count();
    let outcome_missing_events = rows
        .iter()
        .filter(|row| row.status == OptionsExpressionEventStatus::OutcomeMissing)
        .count();

    Ok(OptionsExpressionTupleInspectionResult {
        tuple_index,
        tuple_count,
        tuple_result: selected.result.clone(),
        total_events: rows.len(),
        executed_events,
        candidate_missing_events,
        outcome_missing_events,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options_types::{
        ImportedOptionsDataset, OptionType, OptionsContractSeries, OptionsDatasetManifestV1,
        OptionsExpressionEventStatus, OptionsExpressionExitReason, OptionsFillModelFamily,
        OptionsMoneynessBucket, OptionsPriceRow, OptionsPriceSemantics, OptionsSignalEvent,
        OptionsSignalEventManifestV1, OptionsStaticFilterBundle,
    };

    fn test_dataset() -> ImportedOptionsDataset {
        ImportedOptionsDataset {
            manifest: OptionsDatasetManifestV1 {
                version: 1,
                options_dataset_id: "spy".to_string(),
                root_symbol: "SPY".to_string(),
                provider_name: "manual".to_string(),
                provider_source_surface: "minute_aggregate".to_string(),
                provider_entitlement_profile: None,
                quote_resolution: "1m".to_string(),
                price_semantics: OptionsPriceSemantics::MinuteAggregate,
                calendar_id: "market_hours_utc".to_string(),
                timezone: "UTC".to_string(),
                coverage_start: 1_700_000_000_000,
                coverage_end: 1_700_000_240_000,
                normalization_version: 1,
                quote_semantics_version: 1,
                corporate_action_mapping_version: 1,
                contract_reference_source: "manual".to_string(),
                contract_reference_as_of_utc: None,
                underlier_reference_source: "underlying_dataset".to_string(),
                dataset_path: "mem".to_string(),
                row_count: 8,
                contract_count: 2,
                aggregate_price_field: Some("close".to_string()),
            },
            contracts: vec![
                OptionsContractSeries {
                    root_symbol: "SPY".to_string(),
                    contract_id: "SPY_C_1".to_string(),
                    option_type: OptionType::Call,
                    expiration_date: "2026-04-10".to_string(),
                    expiration_timestamp_ms: 1_700_086_400_000,
                    strike: 101.0,
                    multiplier: 100.0,
                    rows: vec![
                        OptionsPriceRow {
                            timestamp_ms: 1_700_000_060_000,
                            open: 1.0,
                            high: 1.1,
                            low: 0.9,
                            close: 1.0,
                            underlying_price: 100.0,
                            bid: None,
                            ask: None,
                            mid: None,
                            volume: Some(100.0),
                            open_interest: Some(1000.0),
                            quote_age_ms: None,
                            session_id: None,
                        },
                        OptionsPriceRow {
                            timestamp_ms: 1_700_000_120_000,
                            open: 1.0,
                            high: 1.4,
                            low: 0.95,
                            close: 1.3,
                            underlying_price: 100.5,
                            bid: None,
                            ask: None,
                            mid: None,
                            volume: Some(100.0),
                            open_interest: Some(1000.0),
                            quote_age_ms: None,
                            session_id: None,
                        },
                    ],
                },
                OptionsContractSeries {
                    root_symbol: "SPY".to_string(),
                    contract_id: "SPY_P_1".to_string(),
                    option_type: OptionType::Put,
                    expiration_date: "2026-04-10".to_string(),
                    expiration_timestamp_ms: 1_700_086_400_000,
                    strike: 99.0,
                    multiplier: 100.0,
                    rows: vec![
                        OptionsPriceRow {
                            timestamp_ms: 1_700_000_060_000,
                            open: 1.0,
                            high: 1.1,
                            low: 0.9,
                            close: 1.0,
                            underlying_price: 100.0,
                            bid: None,
                            ask: None,
                            mid: None,
                            volume: Some(100.0),
                            open_interest: Some(1000.0),
                            quote_age_ms: None,
                            session_id: None,
                        },
                        OptionsPriceRow {
                            timestamp_ms: 1_700_000_120_000,
                            open: 1.0,
                            high: 1.05,
                            low: 0.7,
                            close: 0.8,
                            underlying_price: 101.0,
                            bid: None,
                            ask: None,
                            mid: None,
                            volume: Some(100.0),
                            open_interest: Some(1000.0),
                            quote_age_ms: None,
                            session_id: None,
                        },
                    ],
                },
            ],
        }
    }

    fn test_manifest() -> OptionsSignalEventManifestV1 {
        OptionsSignalEventManifestV1 {
            version: 1,
            manifest_id: "manifest".to_string(),
            parent_artifact_id: "artifact".to_string(),
            parent_run_kind: "vectorgrid_run_artifact".to_string(),
            source_dataset_identity_hash: "id".to_string(),
            source_content_fingerprint: "fp".to_string(),
            dataset_hash: "hash".to_string(),
            timeframe: "1m".to_string(),
            resample_spec_hash: None,
            adjustment_mode: "raw".to_string(),
            symbol_root: "SPY".to_string(),
            calendar_id: "market_hours_utc".to_string(),
            runtime_dataset_id: None,
            events: vec![
                OptionsSignalEvent {
                    event_id: "a".to_string(),
                    parent_artifact_id: "artifact".to_string(),
                    parent_run_kind: "vectorgrid_run_artifact".to_string(),
                    source_dataset_identity_hash: "id".to_string(),
                    source_content_fingerprint: "fp".to_string(),
                    dataset_hash: "hash".to_string(),
                    timeframe: "1m".to_string(),
                    resample_spec_hash: None,
                    adjustment_mode: "raw".to_string(),
                    symbol_root: "SPY".to_string(),
                    side: SignalSide::Bullish,
                    decision_timestamp: 1_700_000_000_000,
                    earliest_tradable_timestamp: 1_700_000_060_000,
                    decision_bar_index: 0,
                    earliest_tradable_bar_index: 1,
                    raw_underlier_price: 100.0,
                    adjusted_underlier_price: None,
                    base_entry_timestamp: Some(1_700_000_060_000),
                    base_exit_timestamp: Some(1_700_000_120_000),
                    base_exit_bar_index: Some(2),
                    fold: OptionsEvaluationSplit::Search,
                    session_id: None,
                    regime_tag: None,
                    confidence_tag: None,
                    calendar_id: "market_hours_utc".to_string(),
                },
                OptionsSignalEvent {
                    event_id: "b".to_string(),
                    parent_artifact_id: "artifact".to_string(),
                    parent_run_kind: "vectorgrid_run_artifact".to_string(),
                    source_dataset_identity_hash: "id".to_string(),
                    source_content_fingerprint: "fp".to_string(),
                    dataset_hash: "hash".to_string(),
                    timeframe: "1m".to_string(),
                    resample_spec_hash: None,
                    adjustment_mode: "raw".to_string(),
                    symbol_root: "SPY".to_string(),
                    side: SignalSide::Bearish,
                    decision_timestamp: 1_700_000_120_000,
                    earliest_tradable_timestamp: 1_700_000_060_000,
                    decision_bar_index: 2,
                    earliest_tradable_bar_index: 1,
                    raw_underlier_price: 100.0,
                    adjusted_underlier_price: None,
                    base_entry_timestamp: Some(1_700_000_060_000),
                    base_exit_timestamp: Some(1_700_000_120_000),
                    base_exit_bar_index: Some(2),
                    fold: OptionsEvaluationSplit::Search,
                    session_id: None,
                    regime_tag: None,
                    confidence_tag: None,
                    calendar_id: "market_hours_utc".to_string(),
                },
                OptionsSignalEvent {
                    event_id: "c".to_string(),
                    parent_artifact_id: "artifact".to_string(),
                    parent_run_kind: "vectorgrid_run_artifact".to_string(),
                    source_dataset_identity_hash: "id".to_string(),
                    source_content_fingerprint: "fp".to_string(),
                    dataset_hash: "hash".to_string(),
                    timeframe: "1m".to_string(),
                    resample_spec_hash: None,
                    adjustment_mode: "raw".to_string(),
                    symbol_root: "SPY".to_string(),
                    side: SignalSide::Bullish,
                    decision_timestamp: 1_700_000_240_000,
                    earliest_tradable_timestamp: 1_700_000_060_000,
                    decision_bar_index: 4,
                    earliest_tradable_bar_index: 1,
                    raw_underlier_price: 100.0,
                    adjusted_underlier_price: None,
                    base_entry_timestamp: Some(1_700_000_060_000),
                    base_exit_timestamp: Some(1_700_000_120_000),
                    base_exit_bar_index: Some(2),
                    fold: OptionsEvaluationSplit::Search,
                    session_id: None,
                    regime_tag: None,
                    confidence_tag: None,
                    calendar_id: "market_hours_utc".to_string(),
                },
            ],
        }
    }

    #[test]
    fn options_expression_cpu_reference_scores_tuples() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
        };

        let result =
            run_options_expression_blocking(req, &test_manifest(), &test_dataset()).expect("run");
        assert_eq!(result.backend_used, "cpu_reference");
        assert!(result.tuple_count > 0);
        assert!(!result.ranked_results.is_empty());
    }

    #[test]
    fn candidate_and_exit_summary_compilers_build_dense_stage_buffers() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            delay_minutes: vec![0, 5],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm, OptionsMoneynessBucket::Otm1],
            hold_minutes: vec![1, 5],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
        };
        let dataset = test_dataset();
        let events = assign_splits(
            &test_manifest(),
            req.split_ratio_search,
            req.split_ratio_holdout,
        )
        .expect("events");
        let candidate_axis = CandidateAxis::from_request(&req).expect("candidate axis");
        let exit_axis = ExitAxis::from_request(&req).expect("exit axis");
        let candidate_cache = compile_structural_candidate_cache(
            &events,
            &dataset,
            candidate_axis.clone(),
            req.fill_model,
            &req.filters,
            AggregateStressMode::Baseline,
        );
        assert_eq!(
            candidate_cache.slots.len(),
            events.len() * candidate_axis.slot_count_per_event()
        );
        assert!(candidate_cache.slots.iter().any(|entry| entry.is_some()));

        let exit_cache = compile_exit_summary_cache(
            &events,
            &dataset,
            &candidate_cache,
            exit_axis.clone(),
            AggregateStressMode::Baseline,
        );
        assert_eq!(
            exit_cache.outcomes.len(),
            candidate_cache.slots.len() * exit_axis.summary_count_per_candidate()
        );
        assert!(exit_cache.outcomes.iter().any(|entry| entry.is_some()));
    }

    #[test]
    fn options_expression_run_populates_staging_stats() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
        };

        let result =
            run_options_expression_blocking(req, &test_manifest(), &test_dataset()).expect("run");
        let staging = result.staging.expect("staging");
        assert_eq!(staging.candidate_slot_count, 3);
        assert_eq!(staging.compiled_candidate_count, 3);
        assert_eq!(staging.baseline_exit_summary_count, 3);
        assert_eq!(staging.conservative_exit_summary_count, 3);
    }

    #[test]
    fn options_expression_feasibility_report_tracks_density_and_invalid_reason_totals() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            delay_minutes: vec![0],
            dte_buckets: vec![1, 9],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
        };

        let report =
            analyze_options_expression_feasibility_blocking(req, &test_manifest(), &test_dataset())
                .expect("report");

        assert_eq!(
            report.candidate_slot_count,
            report.compiled_candidate_count + report.invalid_slot_count
        );
        assert_eq!(report.compiled_tuple_count, 2);
        assert!(report.candidate_density >= 0.0);
        assert!(report.invalid_slot_rate >= 0.0);
        assert_eq!(
            report
                .invalid_reasons
                .iter()
                .map(|entry| entry.count)
                .sum::<usize>(),
            report.invalid_slot_count
        );
        assert!(report
            .invalid_reasons
            .iter()
            .any(|entry| entry.reason == OptionsCandidateInvalidReason::NoExpirationInBucket));
        assert_eq!(report.event_coverage.total_events_with_candidates, 3);
        assert_eq!(report.event_coverage.search_events_with_candidates, 1);
        assert_eq!(report.event_coverage.total_events_with_baseline_outcomes, 3);
        assert_eq!(
            report
                .event_coverage
                .total_events_with_conservative_outcomes,
            3
        );
        assert!(report.structural_cache_bytes_estimate > 0);
        assert!(report.baseline_summary_bytes_estimate > 0);
        assert!(report.conservative_summary_bytes_estimate > 0);
    }

    #[test]
    fn tuple_inspection_returns_event_rows_for_ranked_tuple() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
        };

        let inspection =
            inspect_options_expression_tuple_blocking(req, 0, &test_manifest(), &test_dataset())
                .expect("inspection");

        assert_eq!(inspection.tuple_index, 0);
        assert_eq!(inspection.total_events, 3);
        assert_eq!(
            inspection.executed_events
                + inspection.candidate_missing_events
                + inspection.outcome_missing_events,
            inspection.total_events
        );
        assert!(inspection.executed_events >= 1);
        assert_eq!(inspection.outcome_missing_events, 0);
        assert_eq!(inspection.rows[0].event_id, "a");
        assert_eq!(
            inspection.rows[0].status,
            OptionsExpressionEventStatus::Executed
        );
        assert_eq!(inspection.rows[0].contract_id.as_deref(), Some("SPY_C_1"));
        assert_eq!(
            inspection.rows[0].exit_reason,
            Some(OptionsExpressionExitReason::TargetHit)
        );
        assert!(inspection.rows.iter().all(|row| !row.event_id.is_empty()));
    }
}
