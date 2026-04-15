use crate::options_types::{
    ImportedOptionsDataset, OptionType, OptionsCandidateAxisCoverage,
    OptionsCandidateInvalidReason, OptionsCandidateInvalidReasonCount, OptionsContractSeries,
    OptionsDelayBucketCoverage, OptionsDteBucketCoverage, OptionsEvaluationSplit,
    OptionsExitTemplateFamily, OptionsExpressionAutoBackendDecision,
    OptionsExpressionCoverageAdequacySummary, OptionsExpressionCoverageAdequacyVerdict,
    OptionsExpressionDiagnostics, OptionsExpressionEventDetailRow,
    OptionsExpressionEventPathInspectionResult, OptionsExpressionEventPathPoint,
    OptionsExpressionEventStatus, OptionsExpressionExitReason,
    OptionsExpressionFeasibilityCacheStats, OptionsExpressionFeasibilityEventCoverage,
    OptionsExpressionFeasibilityReport, OptionsExpressionFeasibilityTimingStats,
    OptionsExpressionMetrics, OptionsExpressionOverlapWindowSummary, OptionsExpressionRunRequest,
    OptionsExpressionRunResult, OptionsExpressionSplitInvalidReasonSummary,
    OptionsExpressionStagingStats, OptionsExpressionTupleInspectionResult,
    OptionsExpressionTupleResult, OptionsFillModelFamily, OptionsMoneynessBucket,
    OptionsMoneynessBucketCoverage, OptionsPriceRow, OptionsPriceSemantics, OptionsSignalEvent,
    OptionsSignalEventManifestV1, OptionsSplitAssignmentMode, OptionsStructureFamily,
    OptionsTupleSpec, SignalSide,
};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::cuda_device::preferred_cuda_device_id;
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::options_expression_gpu::{
    inspect_options_expression_gpu_combined_plan, inspect_options_expression_gpu_plan,
    score_options_expression_tensors_gpu_multi_range, score_options_expression_tensors_gpu_range,
    score_options_expression_tensors_gpu_search_holdout_stress, OptionsGpuRuntimeCacheUsage,
    OptionsGpuTupleAggregate,
};
#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
use crate::options_types::OptionsExpressionBackendPreference;

const MINUTE_MS: i64 = 60_000;
const DAY_MS: i64 = 86_400_000;
const OPTIONS_EXPRESSION_PREPARE_CACHE_MAX_ENTRIES: usize = 32;
// Keep runtime `auto` aligned with the current Phase 0.5 product gate so
// representative provider-backed slices that already clear the documented
// promotion envelope are not downgraded back to CPU solely by stricter live
// density/invalid heuristics.
const OPTIONS_AUTO_GPU_MIN_CANDIDATE_DENSITY: f64 = 0.25;
const OPTIONS_AUTO_GPU_MAX_INVALID_SLOT_RATE: f64 = 0.75;
const OPTIONS_AUTO_GPU_MIN_COMPILED_TUPLES: usize = 256;
const OPTIONS_AUTO_GPU_MIN_SEARCH_EVENTS: usize = 8;
const OPTIONS_AUTO_GPU_MIN_PROJECTED_EVENT_TUPLES: u64 = 10_000;
const OPTIONS_AUTO_GPU_MIN_MEASURED_SPEEDUP: f64 = 1.25;
const OPTIONS_MIN_HEALTHY_SEARCH_OVERLAP_RATE: f64 = 0.75;
const OPTIONS_MIN_HEALTHY_TOTAL_OVERLAP_RATE: f64 = 0.75;
const OPTIONS_MIN_HEALTHY_SEARCH_CANDIDATE_COVERAGE_RATE: f64 = 0.50;
const OPTIONS_MIN_HEALTHY_SEARCH_BASELINE_OUTCOME_COVERAGE_RATE: f64 = 0.50;
const OPTIONS_MIN_HEALTHY_HOLDOUT_COVERAGE_RATE: f64 = 0.50;
const OPTIONS_MIN_HEALTHY_SEALED_COVERAGE_RATE: f64 = 0.50;
const OPTIONS_DOMINANT_INVALID_REASON_SHARE_THRESHOLD: f64 = 0.40;

#[cfg(test)]
fn forced_options_expression_cuda_device_id() -> &'static Mutex<Option<Option<u32>>> {
    static FORCED: OnceLock<Mutex<Option<Option<u32>>>> = OnceLock::new();
    FORCED.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn forced_options_expression_gpu_plan_error() -> &'static Mutex<Option<String>> {
    static FORCED: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    FORCED.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn set_forced_options_expression_cuda_device_id_for_test(value: Option<Option<u32>>) {
    *forced_options_expression_cuda_device_id()
        .lock()
        .expect("forced cuda device override poisoned") = value;
}

#[cfg(test)]
fn set_forced_options_expression_gpu_plan_error_for_test(value: Option<String>) {
    *forced_options_expression_gpu_plan_error()
        .lock()
        .expect("forced gpu plan override poisoned") = value;
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn current_preferred_cuda_device_id() -> Option<u32> {
    #[cfg(test)]
    {
        if let Some(forced) = forced_options_expression_cuda_device_id()
            .lock()
            .expect("forced cuda device override poisoned")
            .clone()
        {
            return forced;
        }
    }
    preferred_cuda_device_id()
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn inspect_gpu_plan_for_feasibility(
    device_id: u32,
    baseline_inputs: &GpuFlatScoreInputs,
    conservative_inputs: Option<&GpuFlatScoreInputs>,
    harsher_inputs: Option<&GpuFlatScoreInputs>,
) -> Result<OptionsGpuRuntimeCacheUsage, String> {
    #[cfg(test)]
    {
        if let Some(reason) = forced_options_expression_gpu_plan_error()
            .lock()
            .expect("forced gpu plan override poisoned")
            .clone()
        {
            return Err(reason);
        }
    }
    if options_gpu_combined_scoring_enabled() {
        if let (Some(conservative_inputs), Some(harsher_inputs)) =
            (conservative_inputs, harsher_inputs)
        {
            if let Ok(usage) = inspect_options_expression_gpu_combined_plan(
                device_id,
                baseline_inputs,
                conservative_inputs,
                harsher_inputs,
            ) {
                return Ok(usage);
            }
        }
    }
    inspect_options_expression_gpu_plan(device_id, baseline_inputs)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AggregateStressMode {
    Baseline,
    Conservative,
    Harsher,
    Extreme,
}

#[derive(Clone, Copy, Debug)]
struct SelectedLeg {
    contract_index: usize,
    entry_index: usize,
}

#[derive(Clone, Copy, Debug)]
struct SelectedCandidate {
    primary_leg: SelectedLeg,
    companion_leg: Option<SelectedLeg>,
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
    companion_entry_price: Option<f64>,
    entry_underlier_price: f64,
    entry_index: usize,
    exit_timestamp_ms: i64,
    exit_price: f64,
    companion_exit_price: Option<f64>,
    exit_index: usize,
    premium_paid: f64,
    pnl_per_contract: f64,
    return_on_premium: f64,
    path_option_high: f64,
    path_option_low: f64,
    path_underlier_high: f64,
    path_underlier_low: f64,
    exit_reason: OptionsExpressionExitReason,
}

impl DetailedTradeOutcome {
    #[cfg(test)]
    fn summary(&self) -> TradeOutcome {
        TradeOutcome {
            pnl_per_contract: self.pnl_per_contract,
            premium_paid: self.premium_paid,
            return_on_premium: self.return_on_premium,
        }
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[derive(Clone, Debug)]
pub(crate) struct GpuFlatScoreInputs {
    pub(crate) tuple_candidate_offsets: Vec<u32>,
    pub(crate) tuple_summary_offsets: Vec<u32>,
    pub(crate) outcome_valid: Vec<u8>,
    pub(crate) outcome_pnl: Vec<f32>,
    pub(crate) outcome_premium: Vec<f32>,
    pub(crate) candidate_slot_stride: usize,
    pub(crate) summary_stride: usize,
    pub(crate) shared_key: String,
    pub(crate) outcome_key: String,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn hash_offset_layout(
    candidate_slot_stride: usize,
    summary_stride: usize,
    tuple_candidate_offsets: &[u32],
    tuple_summary_offsets: &[u32],
) -> u64 {
    let mut hasher = DefaultHasher::new();
    candidate_slot_stride.hash(&mut hasher);
    summary_stride.hash(&mut hasher);
    tuple_candidate_offsets.hash(&mut hasher);
    tuple_summary_offsets.hash(&mut hasher);
    hasher.finish()
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn build_gpu_flat_score_inputs(
    compiled_tuples: &[CompiledTupleSpec],
    candidate_cache: &StructuralCandidateCache,
    exit_summary_cache: &ExitSummaryCache,
    outcome_key_seed: &str,
) -> Result<GpuFlatScoreInputs, String> {
    let tuple_candidate_offsets = compiled_tuples
        .iter()
        .map(|tuple| {
            u32::try_from(candidate_cache.axis.local_slot_offset(
                tuple.delay_index,
                tuple.dte_index,
                tuple.moneyness_index,
            ))
            .map_err(|_| "tuple candidate offset exceeds u32::MAX".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tuple_summary_offsets = compiled_tuples
        .iter()
        .map(|tuple| {
            u32::try_from(exit_summary_cache.axis.local_summary_offset(
                tuple.hold_index,
                tuple.target_index,
                tuple.stop_index,
            ))
            .map_err(|_| "tuple summary offset exceeds u32::MAX".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut outcome_valid = Vec::with_capacity(exit_summary_cache.outcomes.len());
    let mut outcome_pnl = Vec::with_capacity(exit_summary_cache.outcomes.len());
    let mut outcome_premium = Vec::with_capacity(exit_summary_cache.outcomes.len());
    for outcome in &exit_summary_cache.outcomes {
        if let Some(outcome) = outcome {
            outcome_valid.push(1u8);
            outcome_pnl.push(outcome.pnl_per_contract as f32);
            outcome_premium.push(outcome.premium_paid as f32);
        } else {
            outcome_valid.push(0u8);
            outcome_pnl.push(0.0f32);
            outcome_premium.push(0.0f32);
        }
    }
    let candidate_slot_stride = candidate_cache.axis.slot_count_per_event();
    let summary_stride = exit_summary_cache.axis.summary_count_per_candidate();
    let shared_key = format!(
        "offset_layout:{}",
        hash_offset_layout(
            candidate_slot_stride,
            summary_stride,
            &tuple_candidate_offsets,
            &tuple_summary_offsets
        )
    );
    Ok(GpuFlatScoreInputs {
        tuple_candidate_offsets,
        tuple_summary_offsets,
        outcome_valid,
        outcome_pnl,
        outcome_premium,
        candidate_slot_stride,
        summary_stride,
        shared_key,
        outcome_key: outcome_key_seed.to_string(),
    })
}

static STRUCTURAL_CANDIDATE_CACHE: OnceLock<Mutex<BTreeMap<String, StructuralCandidateCache>>> =
    OnceLock::new();
static EXIT_SUMMARY_CACHE: OnceLock<Mutex<BTreeMap<String, ExitSummaryCache>>> = OnceLock::new();

fn structural_candidate_cache_store() -> &'static Mutex<BTreeMap<String, StructuralCandidateCache>>
{
    STRUCTURAL_CANDIDATE_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn exit_summary_cache_store() -> &'static Mutex<BTreeMap<String, ExitSummaryCache>> {
    EXIT_SUMMARY_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()))
}

#[cfg(test)]
fn reset_options_expression_prepare_caches() {
    if let Some(cache) = STRUCTURAL_CANDIDATE_CACHE.get() {
        cache
            .lock()
            .expect("structural candidate cache poisoned")
            .clear();
    }
    if let Some(cache) = EXIT_SUMMARY_CACHE.get() {
        cache.lock().expect("exit summary cache poisoned").clear();
    }
}

fn evict_if_needed<T>(cache: &mut BTreeMap<String, T>) {
    if cache.len() >= OPTIONS_EXPRESSION_PREPARE_CACHE_MAX_ENTRIES {
        if let Some(first_key) = cache.keys().next().cloned() {
            cache.remove(&first_key);
        }
    }
}

fn stress_mode_label(stress_mode: AggregateStressMode) -> &'static str {
    match stress_mode {
        AggregateStressMode::Baseline => "baseline",
        AggregateStressMode::Conservative => "conservative",
        AggregateStressMode::Harsher => "harsher",
        AggregateStressMode::Extreme => "extreme",
    }
}

fn hash_signal_manifest_cache_inputs(signal_manifest: &OptionsSignalEventManifestV1) -> u64 {
    let mut hasher = DefaultHasher::new();
    signal_manifest.manifest_id.hash(&mut hasher);
    signal_manifest.parent_artifact_id.hash(&mut hasher);
    signal_manifest.parent_run_kind.hash(&mut hasher);
    signal_manifest
        .source_dataset_identity_hash
        .hash(&mut hasher);
    signal_manifest.source_content_fingerprint.hash(&mut hasher);
    signal_manifest.dataset_hash.hash(&mut hasher);
    signal_manifest.timeframe.hash(&mut hasher);
    signal_manifest.resample_spec_hash.hash(&mut hasher);
    signal_manifest.adjustment_mode.hash(&mut hasher);
    signal_manifest.symbol_root.hash(&mut hasher);
    signal_manifest.calendar_id.hash(&mut hasher);
    signal_manifest.runtime_dataset_id.hash(&mut hasher);
    signal_manifest.events.len().hash(&mut hasher);
    for event in &signal_manifest.events {
        event.event_id.hash(&mut hasher);
        std::mem::discriminant(&event.side).hash(&mut hasher);
        event.decision_timestamp.hash(&mut hasher);
        event.earliest_tradable_timestamp.hash(&mut hasher);
        event.decision_bar_index.hash(&mut hasher);
        event.earliest_tradable_bar_index.hash(&mut hasher);
        event.raw_underlier_price.to_bits().hash(&mut hasher);
        event
            .adjusted_underlier_price
            .map(f64::to_bits)
            .hash(&mut hasher);
        event.base_entry_timestamp.hash(&mut hasher);
        event.base_exit_timestamp.hash(&mut hasher);
        event.base_exit_bar_index.hash(&mut hasher);
        std::mem::discriminant(&event.fold).hash(&mut hasher);
        event.session_id.hash(&mut hasher);
        event.regime_tag.hash(&mut hasher);
        event.confidence_tag.hash(&mut hasher);
        event.calendar_id.hash(&mut hasher);
    }
    hasher.finish()
}

fn structural_candidate_cache_key(
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
    axis: &CandidateAxis,
    fill_model: OptionsFillModelFamily,
    filters: &crate::options_types::OptionsStaticFilterBundle,
    stress_mode: AggregateStressMode,
) -> String {
    let signal_manifest_cache_hash = hash_signal_manifest_cache_inputs(signal_manifest);
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        signal_manifest.manifest_id,
        signal_manifest.source_content_fingerprint,
        signal_manifest.dataset_hash,
        signal_manifest.events.len(),
        signal_manifest_cache_hash,
        dataset.manifest.options_dataset_id,
        dataset.manifest.root_symbol,
        dataset.manifest.row_count,
        dataset.manifest.contract_count,
        dataset.manifest.coverage_start,
        dataset.manifest.coverage_end,
        dataset.manifest.normalization_version,
        dataset.manifest.quote_semantics_version,
        dataset.manifest.corporate_action_mapping_version,
        dataset.manifest.price_semantics,
        dataset.manifest.contract_reference_as_of_utc,
        dataset.manifest.aggregate_price_field,
        axis.structure_family,
        axis.delay_minutes,
        axis.dte_days,
        axis.moneyness_buckets,
        fill_model,
        filters,
        stress_mode_label(stress_mode),
    )
}

fn exit_summary_cache_key(structural_key: &str, axis: &ExitAxis) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        structural_key,
        axis.hold_minutes,
        axis.target_values,
        axis.stop_values,
        axis.exit_template,
        axis.fill_model,
    )
}

#[derive(Clone, Debug)]
struct CacheLookup<T> {
    value: T,
    hit: bool,
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
    if price_semantics.supports_fill_model(fill_model) {
        Ok(())
    } else {
        Err(format!(
            "fill model '{fill_model:?}' is incompatible with price semantics '{price_semantics:?}'. allowed fill models: {}",
            price_semantics.allowed_fill_models_label()
        ))
    }
}

fn assign_splits(
    manifest: &OptionsSignalEventManifestV1,
    search_ratio: f64,
    holdout_ratio: f64,
    split_assignment_mode: OptionsSplitAssignmentMode,
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

    fn boundaries(total: usize, search_ratio: f64, holdout_ratio: f64) -> (usize, usize) {
        let mut search_end = ((total as f64) * search_ratio).floor() as usize;
        let mut holdout_len = ((total as f64) * holdout_ratio).floor() as usize;
        search_end = search_end.clamp(1, total.saturating_sub(2));
        holdout_len = holdout_len.clamp(1, total.saturating_sub(search_end + 1));
        let holdout_end = (search_end + holdout_len).min(total.saturating_sub(1));
        (search_end, holdout_end)
    }

    match split_assignment_mode {
        OptionsSplitAssignmentMode::EventChronological => {
            let (search_end, holdout_end) = boundaries(total, search_ratio, holdout_ratio);
            for (idx, event) in events.iter_mut().enumerate() {
                event.fold = if idx < search_end {
                    OptionsEvaluationSplit::Search
                } else if idx < holdout_end {
                    OptionsEvaluationSplit::Holdout
                } else {
                    OptionsEvaluationSplit::SealedTest
                };
            }
        }
        OptionsSplitAssignmentMode::SessionChronological => {
            let mut ordered_sessions: Vec<String> = Vec::new();
            let mut session_index_by_key = BTreeMap::<String, usize>::new();
            let mut event_session_indices = Vec::with_capacity(events.len());
            for event in &events {
                let key = event
                    .session_id
                    .as_ref()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| {
                        format!(
                            "utc_day:{}",
                            event.earliest_tradable_timestamp.div_euclid(DAY_MS)
                        )
                    });
                let next_index = if let Some(existing) = session_index_by_key.get(&key) {
                    *existing
                } else {
                    let index = ordered_sessions.len();
                    ordered_sessions.push(key.clone());
                    session_index_by_key.insert(key, index);
                    index
                };
                event_session_indices.push(next_index);
            }

            if ordered_sessions.len() < 3 {
                let (search_end, holdout_end) = boundaries(total, search_ratio, holdout_ratio);
                for (idx, event) in events.iter_mut().enumerate() {
                    event.fold = if idx < search_end {
                        OptionsEvaluationSplit::Search
                    } else if idx < holdout_end {
                        OptionsEvaluationSplit::Holdout
                    } else {
                        OptionsEvaluationSplit::SealedTest
                    };
                }
            } else {
                let (search_session_end, holdout_session_end) =
                    boundaries(ordered_sessions.len(), search_ratio, holdout_ratio);
                for (event, session_index) in
                    events.iter_mut().zip(event_session_indices.into_iter())
                {
                    event.fold = if session_index < search_session_end {
                        OptionsEvaluationSplit::Search
                    } else if session_index < holdout_session_end {
                        OptionsEvaluationSplit::Holdout
                    } else {
                        OptionsEvaluationSplit::SealedTest
                    };
                }
            }
        }
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
        AggregateStressMode::Harsher => row.high.max(row.close) * 1.02,
        AggregateStressMode::Extreme => row.high.max(row.close) * 1.05,
    }
}

fn aggregate_exit_price(row: &OptionsPriceRow, stress_mode: AggregateStressMode) -> f64 {
    match stress_mode {
        AggregateStressMode::Baseline => row.close,
        AggregateStressMode::Conservative => row.low.min(row.close),
        AggregateStressMode::Harsher => (row.low.min(row.close) * 0.98).max(0.0),
        AggregateStressMode::Extreme => (row.low.min(row.close) * 0.95).max(0.0),
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

fn option_exit_price_for_row(
    row: &OptionsPriceRow,
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
    stress_mode: AggregateStressMode,
) -> Option<f64> {
    match price_semantics {
        OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
            Some(aggregate_exit_price(row, stress_mode))
        }
        OptionsPriceSemantics::TopOfBookQuote => quote_exit_price(row, fill_model),
    }
}

fn position_entry_prices(
    structure_family: OptionsStructureFamily,
    primary_row: &OptionsPriceRow,
    companion_row: Option<&OptionsPriceRow>,
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
    stress_mode: AggregateStressMode,
) -> Option<(f64, Option<f64>, f64)> {
    let primary_entry = option_entry_price_for_row(primary_row, price_semantics, fill_model)?;
    if let Some(companion_row) = companion_row {
        match structure_family {
            OptionsStructureFamily::LongSingle => Some((primary_entry, None, primary_entry)),
            OptionsStructureFamily::DebitVertical => {
                let companion_credit = option_exit_price_for_row(
                    companion_row,
                    price_semantics,
                    fill_model,
                    stress_mode,
                )?;
                let net_debit = (primary_entry - companion_credit).max(0.0);
                Some((net_debit, Some(companion_credit), net_debit))
            }
            OptionsStructureFamily::LongPair => {
                let companion_entry =
                    option_entry_price_for_row(companion_row, price_semantics, fill_model)?;
                let net_debit = (primary_entry + companion_entry).max(0.0);
                Some((net_debit, Some(companion_entry), net_debit))
            }
        }
    } else {
        Some((primary_entry, None, primary_entry))
    }
}

fn position_exit_prices(
    structure_family: OptionsStructureFamily,
    primary_row: &OptionsPriceRow,
    companion_row: Option<&OptionsPriceRow>,
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
    stress_mode: AggregateStressMode,
) -> Option<(f64, Option<f64>, f64)> {
    let primary_exit =
        option_exit_price_for_row(primary_row, price_semantics, fill_model, stress_mode)?;
    if let Some(companion_row) = companion_row {
        match structure_family {
            OptionsStructureFamily::LongSingle => Some((primary_exit, None, primary_exit)),
            OptionsStructureFamily::DebitVertical => {
                let companion_debit =
                    option_entry_price_for_row(companion_row, price_semantics, fill_model)?;
                let net_exit = (primary_exit - companion_debit).max(0.0);
                Some((net_exit, Some(companion_debit), net_exit))
            }
            OptionsStructureFamily::LongPair => {
                let companion_exit = option_exit_price_for_row(
                    companion_row,
                    price_semantics,
                    fill_model,
                    stress_mode,
                )?;
                let net_exit = (primary_exit + companion_exit).max(0.0);
                Some((net_exit, Some(companion_exit), net_exit))
            }
        }
    } else {
        Some((primary_exit, None, primary_exit))
    }
}

fn option_path_range_for_row(
    row: &OptionsPriceRow,
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
    stress_mode: AggregateStressMode,
) -> Option<(f64, f64)> {
    match price_semantics {
        OptionsPriceSemantics::MinuteAggregate | OptionsPriceSemantics::DerivedMidSnapshot => {
            let high = match stress_mode {
                AggregateStressMode::Baseline => row.high.max(row.close),
                AggregateStressMode::Conservative => row.high.max(row.close),
                AggregateStressMode::Harsher => row.high.max(row.close) * 1.02,
                AggregateStressMode::Extreme => row.high.max(row.close) * 1.05,
            };
            let low = match stress_mode {
                AggregateStressMode::Baseline => row.low.min(row.close),
                AggregateStressMode::Conservative => row.low.min(row.close),
                AggregateStressMode::Harsher => (row.low.min(row.close) * 0.98).max(0.0),
                AggregateStressMode::Extreme => (row.low.min(row.close) * 0.95).max(0.0),
            };
            Some((high, low))
        }
        OptionsPriceSemantics::TopOfBookQuote => {
            let px = quote_exit_price(row, fill_model)?;
            Some((px, px))
        }
    }
}

fn position_path_range_for_rows(
    structure_family: OptionsStructureFamily,
    primary_row: &OptionsPriceRow,
    companion_row: Option<&OptionsPriceRow>,
    price_semantics: OptionsPriceSemantics,
    fill_model: OptionsFillModelFamily,
    stress_mode: AggregateStressMode,
) -> Option<(f64, f64)> {
    let (primary_high, primary_low) =
        option_path_range_for_row(primary_row, price_semantics, fill_model, stress_mode)?;
    if let Some(companion_row) = companion_row {
        let (companion_high, companion_low) =
            option_path_range_for_row(companion_row, price_semantics, fill_model, stress_mode)?;
        match structure_family {
            OptionsStructureFamily::LongSingle => Some((primary_high, primary_low)),
            OptionsStructureFamily::DebitVertical => Some((
                (primary_high - companion_low).max(0.0),
                (primary_low - companion_high).max(0.0),
            )),
            OptionsStructureFamily::LongPair => Some((
                (primary_high + companion_high).max(0.0),
                (primary_low + companion_low).max(0.0),
            )),
        }
    } else {
        Some((primary_high, primary_low))
    }
}

fn find_companion_contract_index(
    index: &IndexedOptionsDataset<'_>,
    primary_contract_index: usize,
    side: SignalSide,
) -> Option<usize> {
    let primary = index.contract(primary_contract_index);
    let mut best: Option<(f64, usize)> = None;
    for &candidate_index in index.contracts_for_side(side) {
        if candidate_index == primary_contract_index {
            continue;
        }
        let candidate = index.contract(candidate_index);
        if candidate.expiration_timestamp_ms != primary.expiration_timestamp_ms {
            continue;
        }
        let strike_distance = match side {
            SignalSide::Bullish => candidate.strike - primary.strike,
            SignalSide::Bearish => primary.strike - candidate.strike,
        };
        if strike_distance <= 0.0 {
            continue;
        }
        match best {
            Some((best_distance, _)) if strike_distance >= best_distance => {}
            _ => best = Some((strike_distance, candidate_index)),
        }
    }
    best.map(|(_, index)| index)
}

fn align_entry_indexes(
    primary_rows: &[OptionsPriceRow],
    companion_rows: &[OptionsPriceRow],
    requested_entry_ts: i64,
) -> Option<(usize, usize)> {
    let mut target_ts = requested_entry_ts;
    loop {
        let primary_index = lower_bound_timestamp(primary_rows, target_ts)?;
        let companion_index = lower_bound_timestamp(companion_rows, target_ts)?;
        let primary_ts = primary_rows.get(primary_index)?.timestamp_ms;
        let companion_ts = companion_rows.get(companion_index)?.timestamp_ms;
        if primary_ts == companion_ts {
            return Some((primary_index, companion_index));
        }
        target_ts = primary_ts.max(companion_ts);
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
    let (entry_index, companion_leg, premium_paid) = match tuple.structure_family {
        OptionsStructureFamily::LongSingle => {
            let Some(entry_index) = lower_bound_timestamp(&contract.rows, requested_entry_ts)
            else {
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
            let (_, _, entry_price) = position_entry_prices(
                tuple.structure_family,
                entry_row,
                None,
                price_semantics,
                tuple.fill_model,
                stress_mode,
            )
            .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            (entry_index, None, entry_price * contract.multiplier)
        }
        OptionsStructureFamily::DebitVertical | OptionsStructureFamily::LongPair => {
            let Some(companion_contract_index) =
                find_companion_contract_index(index, contract_index, event.side)
            else {
                return Err(OptionsCandidateInvalidReason::NoCompanionLegInStructure);
            };
            let companion_contract = index.contract(companion_contract_index);
            let Some((primary_entry_index, companion_entry_index)) =
                align_entry_indexes(&contract.rows, &companion_contract.rows, requested_entry_ts)
            else {
                return Err(OptionsCandidateInvalidReason::NoPriceRowAfterEntry);
            };
            let entry_timestamp_ms = contract
                .rows
                .get(primary_entry_index)
                .map(|row| row.timestamp_ms)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            if let Some(max_lag_minutes) = filters.max_entry_lag_minutes {
                let max_lag_ms = i64::from(max_lag_minutes) * MINUTE_MS;
                if entry_timestamp_ms - requested_entry_ts > max_lag_ms {
                    return Err(OptionsCandidateInvalidReason::EntryPastCutoff);
                }
            }
            let primary_row = contract
                .rows
                .get(primary_entry_index)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            let companion_row = companion_contract
                .rows
                .get(companion_entry_index)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            let (_, _, entry_price) = position_entry_prices(
                tuple.structure_family,
                primary_row,
                Some(companion_row),
                price_semantics,
                tuple.fill_model,
                stress_mode,
            )
            .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            (
                primary_entry_index,
                Some(SelectedLeg {
                    contract_index: companion_contract_index,
                    entry_index: companion_entry_index,
                }),
                entry_price * contract.multiplier,
            )
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
    let entry_row = contract
        .rows
        .get(entry_index)
        .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
    if let Some(min_volume) = filters.min_volume {
        if entry_row.volume.unwrap_or_default() < min_volume {
            return Err(OptionsCandidateInvalidReason::InsufficientVolume);
        }
        if let Some(companion_leg) = companion_leg {
            let companion_row = index
                .contract(companion_leg.contract_index)
                .rows
                .get(companion_leg.entry_index)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            if companion_row.volume.unwrap_or_default() < min_volume {
                return Err(OptionsCandidateInvalidReason::InsufficientVolume);
            }
        }
    }
    if let Some(min_open_interest) = filters.min_open_interest {
        if entry_row.open_interest.unwrap_or_default() < min_open_interest {
            return Err(OptionsCandidateInvalidReason::InsufficientOpenInterest);
        }
        if let Some(companion_leg) = companion_leg {
            let companion_row = index
                .contract(companion_leg.contract_index)
                .rows
                .get(companion_leg.entry_index)
                .ok_or(OptionsCandidateInvalidReason::NoPriceRowAfterEntry)?;
            if companion_row.open_interest.unwrap_or_default() < min_open_interest {
                return Err(OptionsCandidateInvalidReason::InsufficientOpenInterest);
            }
        }
    }
    if premium_paid <= 0.0 {
        return Err(OptionsCandidateInvalidReason::PremiumTooLow);
    }
    Ok(SelectedCandidate {
        primary_leg: SelectedLeg {
            contract_index,
            entry_index,
        },
        companion_leg,
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
    let contract = index.contract(candidate.primary_leg.contract_index);
    let contract_rows = &contract.rows;
    let entry_row = contract_rows.get(candidate.primary_leg.entry_index)?;
    let companion_contract = candidate
        .companion_leg
        .map(|leg| index.contract(leg.contract_index));
    let companion_rows = companion_contract.map(|contract| contract.rows.as_slice());
    let companion_entry_row = match (candidate.companion_leg, companion_rows) {
        (Some(leg), Some(rows)) => rows.get(leg.entry_index),
        _ => None,
    };
    let (_entry_price, companion_entry_price, net_entry_price) = position_entry_prices(
        tuple.structure_family,
        entry_row,
        companion_entry_row,
        price_semantics,
        tuple.fill_model,
        stress_mode,
    )?;
    if !net_entry_price.is_finite() || net_entry_price <= 0.0 {
        return None;
    }
    let premium_paid = net_entry_price * contract.multiplier;
    let mut path_option_high = net_entry_price;
    let mut path_option_low = net_entry_price;
    let mut path_underlier_high = entry_row.underlying_price;
    let mut path_underlier_low = entry_row.underlying_price;
    let target_price = tuple
        .target_return
        .map(|pct| net_entry_price * (1.0 + pct.max(0.0)));
    let stop_price = tuple
        .stop_return
        .map(|pct| net_entry_price * (1.0 - pct.max(0.0)));
    let forced_expiry_cutoff = contract.expiration_timestamp_ms.saturating_sub(MINUTE_MS);
    let requested_cutoff = match tuple.exit_template {
        OptionsExitTemplateFamily::FixedHold | OptionsExitTemplateFamily::FixedHoldOrTargetStop => {
            resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)
        }
        OptionsExitTemplateFamily::EndOfDayOrTargetStop => {
            resolve_eod_cutoff(contract_rows, candidate.primary_leg.entry_index)
        }
        OptionsExitTemplateFamily::BaseExitOrTargetStop => event
            .base_exit_timestamp
            .unwrap_or_else(|| resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)),
    };
    let cutoff_timestamp_ms = requested_cutoff.min(forced_expiry_cutoff);

    if let (Some(companion_leg), Some(companion_rows)) = (candidate.companion_leg, companion_rows) {
        let mut primary_index = candidate.primary_leg.entry_index;
        let mut companion_index = companion_leg.entry_index;
        let mut final_pair: Option<(usize, usize)> = None;
        while primary_index < contract_rows.len() && companion_index < companion_rows.len() {
            let primary_row = contract_rows.get(primary_index)?;
            let companion_row = companion_rows.get(companion_index)?;
            match primary_row.timestamp_ms.cmp(&companion_row.timestamp_ms) {
                Ordering::Less => {
                    primary_index += 1;
                    continue;
                }
                Ordering::Greater => {
                    companion_index += 1;
                    continue;
                }
                Ordering::Equal => {
                    let timestamp_ms = primary_row.timestamp_ms;
                    if timestamp_ms > cutoff_timestamp_ms {
                        final_pair = Some((primary_index, companion_index));
                        break;
                    }
                    if timestamp_ms > entry_row.timestamp_ms {
                        let (option_high, option_low) = position_path_range_for_rows(
                            tuple.structure_family,
                            primary_row,
                            Some(companion_row),
                            price_semantics,
                            tuple.fill_model,
                            stress_mode,
                        )?;
                        path_option_high = path_option_high.max(option_high);
                        path_option_low = path_option_low.min(option_low);
                        path_underlier_high = path_underlier_high.max(primary_row.underlying_price);
                        path_underlier_low = path_underlier_low.min(primary_row.underlying_price);
                        let target_hit = target_price
                            .map(|target| option_high >= target)
                            .unwrap_or(false);
                        let stop_hit = stop_price.map(|stop| option_low <= stop).unwrap_or(false);
                        if stop_hit || target_hit {
                            let (_, companion_exit_price, fallback_exit_price) =
                                position_exit_prices(
                                    tuple.structure_family,
                                    primary_row,
                                    Some(companion_row),
                                    price_semantics,
                                    tuple.fill_model,
                                    stress_mode,
                                )?;
                            let exit_price = if stop_hit {
                                stop_price.unwrap_or(fallback_exit_price)
                            } else {
                                target_price.unwrap_or(fallback_exit_price)
                            };
                            let pnl_per_contract =
                                (exit_price - net_entry_price) * contract.multiplier;
                            return Some(DetailedTradeOutcome {
                                entry_timestamp_ms: entry_row.timestamp_ms,
                                entry_price: net_entry_price,
                                companion_entry_price,
                                entry_underlier_price: entry_row.underlying_price,
                                entry_index: candidate.primary_leg.entry_index,
                                exit_timestamp_ms: timestamp_ms,
                                exit_price,
                                companion_exit_price,
                                exit_index: primary_index,
                                pnl_per_contract,
                                premium_paid,
                                return_on_premium: pnl_per_contract / premium_paid,
                                path_option_high,
                                path_option_low,
                                path_underlier_high,
                                path_underlier_low,
                                exit_reason: if stop_hit {
                                    OptionsExpressionExitReason::StopHit
                                } else {
                                    OptionsExpressionExitReason::TargetHit
                                },
                            });
                        }
                    }
                    if timestamp_ms == cutoff_timestamp_ms {
                        final_pair = Some((primary_index, companion_index));
                        break;
                    }
                    primary_index += 1;
                    companion_index += 1;
                }
            }
        }

        let (exit_primary_index, exit_companion_index) = final_pair?;
        let exit_primary_row = contract_rows.get(exit_primary_index)?;
        let exit_companion_row = companion_rows.get(exit_companion_index)?;
        let (_, companion_exit_price, net_exit_price) = position_exit_prices(
            tuple.structure_family,
            exit_primary_row,
            Some(exit_companion_row),
            price_semantics,
            tuple.fill_model,
            stress_mode,
        )?;
        let pnl_per_contract = (net_exit_price - net_entry_price) * contract.multiplier;
        return Some(DetailedTradeOutcome {
            entry_timestamp_ms: entry_row.timestamp_ms,
            entry_price: net_entry_price,
            companion_entry_price,
            entry_underlier_price: entry_row.underlying_price,
            entry_index: candidate.primary_leg.entry_index,
            exit_timestamp_ms: exit_primary_row.timestamp_ms,
            exit_price: net_exit_price,
            companion_exit_price,
            exit_index: exit_primary_index,
            pnl_per_contract,
            premium_paid,
            return_on_premium: pnl_per_contract / premium_paid,
            path_option_high,
            path_option_low,
            path_underlier_high,
            path_underlier_low,
            exit_reason: cutoff_exit_reason(tuple, requested_cutoff, forced_expiry_cutoff),
        });
    }

    let cutoff_index = find_cutoff_index(
        contract_rows,
        candidate.primary_leg.entry_index,
        cutoff_timestamp_ms,
    )?;
    for idx in (candidate.primary_leg.entry_index + 1)..=cutoff_index {
        let row = contract_rows.get(idx)?;
        let (option_high, option_low) = position_path_range_for_rows(
            tuple.structure_family,
            row,
            None,
            price_semantics,
            tuple.fill_model,
            stress_mode,
        )?;
        path_option_high = path_option_high.max(option_high);
        path_option_low = path_option_low.min(option_low);
        path_underlier_high = path_underlier_high.max(row.underlying_price);
        path_underlier_low = path_underlier_low.min(row.underlying_price);
        let target_hit = target_price
            .map(|target| option_high >= target)
            .unwrap_or(false);
        let stop_hit = stop_price.map(|stop| option_low <= stop).unwrap_or(false);
        if stop_hit || target_hit {
            let (_, _, fallback_exit_price) = position_exit_prices(
                tuple.structure_family,
                row,
                None,
                price_semantics,
                tuple.fill_model,
                stress_mode,
            )?;
            let exit_price = if stop_hit {
                stop_price.unwrap_or(fallback_exit_price)
            } else {
                target_price.unwrap_or(fallback_exit_price)
            };
            let pnl_per_contract = (exit_price - net_entry_price) * contract.multiplier;
            return Some(DetailedTradeOutcome {
                entry_timestamp_ms: entry_row.timestamp_ms,
                entry_price: net_entry_price,
                companion_entry_price,
                entry_underlier_price: entry_row.underlying_price,
                entry_index: candidate.primary_leg.entry_index,
                exit_timestamp_ms: row.timestamp_ms,
                exit_price,
                companion_exit_price: None,
                exit_index: idx,
                pnl_per_contract,
                premium_paid,
                return_on_premium: pnl_per_contract / premium_paid,
                path_option_high,
                path_option_low,
                path_underlier_high,
                path_underlier_low,
                exit_reason: if stop_hit {
                    OptionsExpressionExitReason::StopHit
                } else {
                    OptionsExpressionExitReason::TargetHit
                },
            });
        }
    }

    let exit_row = contract_rows.get(cutoff_index)?;
    let (exit_price, companion_exit_price, net_exit_price) = position_exit_prices(
        tuple.structure_family,
        exit_row,
        None,
        price_semantics,
        tuple.fill_model,
        stress_mode,
    )?;
    let pnl_per_contract = (net_exit_price - net_entry_price) * contract.multiplier;
    Some(DetailedTradeOutcome {
        entry_timestamp_ms: entry_row.timestamp_ms,
        entry_price: net_entry_price,
        companion_entry_price,
        entry_underlier_price: entry_row.underlying_price,
        entry_index: candidate.primary_leg.entry_index,
        exit_timestamp_ms: exit_row.timestamp_ms,
        exit_price,
        companion_exit_price,
        exit_index: cutoff_index,
        pnl_per_contract,
        premium_paid,
        return_on_premium: pnl_per_contract / premium_paid,
        path_option_high,
        path_option_low,
        path_underlier_high,
        path_underlier_low,
        exit_reason: cutoff_exit_reason(tuple, requested_cutoff, forced_expiry_cutoff),
    })
}

// Summary-cache compilation only needs PnL/premium/return, not the full path replay payload.
// Keep this logic semantically aligned with evaluate_trade_detail; tests enforce parity.
fn evaluate_trade_summary(
    event: &OptionsSignalEvent,
    tuple: &OptionsTupleSpec,
    candidate: SelectedCandidate,
    index: &IndexedOptionsDataset<'_>,
    price_semantics: OptionsPriceSemantics,
    stress_mode: AggregateStressMode,
) -> Option<TradeOutcome> {
    let contract = index.contract(candidate.primary_leg.contract_index);
    let contract_rows = &contract.rows;
    let entry_row = contract_rows.get(candidate.primary_leg.entry_index)?;
    let companion_contract = candidate
        .companion_leg
        .map(|leg| index.contract(leg.contract_index));
    let companion_rows = companion_contract.map(|contract| contract.rows.as_slice());
    let companion_entry_row = match (candidate.companion_leg, companion_rows) {
        (Some(leg), Some(rows)) => rows.get(leg.entry_index),
        _ => None,
    };
    let (_, _, net_entry_price) = position_entry_prices(
        tuple.structure_family,
        entry_row,
        companion_entry_row,
        price_semantics,
        tuple.fill_model,
        stress_mode,
    )?;
    if !net_entry_price.is_finite() || net_entry_price <= 0.0 {
        return None;
    }
    let premium_paid = net_entry_price * contract.multiplier;
    let target_price = tuple
        .target_return
        .map(|pct| net_entry_price * (1.0 + pct.max(0.0)));
    let stop_price = tuple
        .stop_return
        .map(|pct| net_entry_price * (1.0 - pct.max(0.0)));
    let forced_expiry_cutoff = contract.expiration_timestamp_ms.saturating_sub(MINUTE_MS);
    let requested_cutoff = match tuple.exit_template {
        OptionsExitTemplateFamily::FixedHold | OptionsExitTemplateFamily::FixedHoldOrTargetStop => {
            resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)
        }
        OptionsExitTemplateFamily::EndOfDayOrTargetStop => {
            resolve_eod_cutoff(contract_rows, candidate.primary_leg.entry_index)
        }
        OptionsExitTemplateFamily::BaseExitOrTargetStop => event
            .base_exit_timestamp
            .unwrap_or_else(|| resolve_fixed_hold_cutoff(entry_row.timestamp_ms, tuple)),
    };
    let cutoff_timestamp_ms = requested_cutoff.min(forced_expiry_cutoff);

    if let (Some(companion_leg), Some(companion_rows)) = (candidate.companion_leg, companion_rows) {
        let mut primary_index = candidate.primary_leg.entry_index;
        let mut companion_index = companion_leg.entry_index;
        let mut final_pair: Option<(usize, usize)> = None;
        while primary_index < contract_rows.len() && companion_index < companion_rows.len() {
            let primary_row = contract_rows.get(primary_index)?;
            let companion_row = companion_rows.get(companion_index)?;
            match primary_row.timestamp_ms.cmp(&companion_row.timestamp_ms) {
                Ordering::Less => {
                    primary_index += 1;
                    continue;
                }
                Ordering::Greater => {
                    companion_index += 1;
                    continue;
                }
                Ordering::Equal => {
                    let timestamp_ms = primary_row.timestamp_ms;
                    if timestamp_ms > cutoff_timestamp_ms {
                        final_pair = Some((primary_index, companion_index));
                        break;
                    }
                    if timestamp_ms > entry_row.timestamp_ms {
                        let (option_high, option_low) = position_path_range_for_rows(
                            tuple.structure_family,
                            primary_row,
                            Some(companion_row),
                            price_semantics,
                            tuple.fill_model,
                            stress_mode,
                        )?;
                        let target_hit = target_price
                            .map(|target| option_high >= target)
                            .unwrap_or(false);
                        let stop_hit = stop_price.map(|stop| option_low <= stop).unwrap_or(false);
                        if stop_hit || target_hit {
                            let exit_price = match (stop_hit, stop_price, target_price) {
                                (true, Some(stop), _) => stop,
                                (false, _, Some(target)) => target,
                                _ => {
                                    let (_, _, fallback_exit_price) = position_exit_prices(
                                        tuple.structure_family,
                                        primary_row,
                                        Some(companion_row),
                                        price_semantics,
                                        tuple.fill_model,
                                        stress_mode,
                                    )?;
                                    if stop_hit {
                                        stop_price.unwrap_or(fallback_exit_price)
                                    } else {
                                        target_price.unwrap_or(fallback_exit_price)
                                    }
                                }
                            };
                            let pnl_per_contract =
                                (exit_price - net_entry_price) * contract.multiplier;
                            return Some(TradeOutcome {
                                pnl_per_contract,
                                premium_paid,
                                return_on_premium: pnl_per_contract / premium_paid,
                            });
                        }
                    }
                    if timestamp_ms == cutoff_timestamp_ms {
                        final_pair = Some((primary_index, companion_index));
                        break;
                    }
                    primary_index += 1;
                    companion_index += 1;
                }
            }
        }

        let (exit_primary_index, exit_companion_index) = final_pair?;
        let exit_primary_row = contract_rows.get(exit_primary_index)?;
        let exit_companion_row = companion_rows.get(exit_companion_index)?;
        let (_, _, net_exit_price) = position_exit_prices(
            tuple.structure_family,
            exit_primary_row,
            Some(exit_companion_row),
            price_semantics,
            tuple.fill_model,
            stress_mode,
        )?;
        let pnl_per_contract = (net_exit_price - net_entry_price) * contract.multiplier;
        return Some(TradeOutcome {
            pnl_per_contract,
            premium_paid,
            return_on_premium: pnl_per_contract / premium_paid,
        });
    }

    let cutoff_index = find_cutoff_index(
        contract_rows,
        candidate.primary_leg.entry_index,
        cutoff_timestamp_ms,
    )?;
    for idx in (candidate.primary_leg.entry_index + 1)..=cutoff_index {
        let row = contract_rows.get(idx)?;
        let (option_high, option_low) = position_path_range_for_rows(
            tuple.structure_family,
            row,
            None,
            price_semantics,
            tuple.fill_model,
            stress_mode,
        )?;
        let target_hit = target_price
            .map(|target| option_high >= target)
            .unwrap_or(false);
        let stop_hit = stop_price.map(|stop| option_low <= stop).unwrap_or(false);
        if stop_hit || target_hit {
            let exit_price = match (stop_hit, stop_price, target_price) {
                (true, Some(stop), _) => stop,
                (false, _, Some(target)) => target,
                _ => {
                    let (_, _, fallback_exit_price) = position_exit_prices(
                        tuple.structure_family,
                        row,
                        None,
                        price_semantics,
                        tuple.fill_model,
                        stress_mode,
                    )?;
                    if stop_hit {
                        stop_price.unwrap_or(fallback_exit_price)
                    } else {
                        target_price.unwrap_or(fallback_exit_price)
                    }
                }
            };
            let pnl_per_contract = (exit_price - net_entry_price) * contract.multiplier;
            return Some(TradeOutcome {
                pnl_per_contract,
                premium_paid,
                return_on_premium: pnl_per_contract / premium_paid,
            });
        }
    }

    let exit_row = contract_rows.get(cutoff_index)?;
    let (_, _, net_exit_price) = position_exit_prices(
        tuple.structure_family,
        exit_row,
        None,
        price_semantics,
        tuple.fill_model,
        stress_mode,
    )?;
    let pnl_per_contract = (net_exit_price - net_entry_price) * contract.multiplier;
    Some(TradeOutcome {
        pnl_per_contract,
        premium_paid,
        return_on_premium: pnl_per_contract / premium_paid,
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

fn return_on_premium_mean_and_volatility(
    sum_return_on_premium: f64,
    sumsq_return_on_premium: f64,
    valid_trades: usize,
) -> (f64, f64) {
    if valid_trades == 0 {
        return (0.0, 0.0);
    }

    let count = valid_trades as f64;
    let mean_return_on_premium = sum_return_on_premium / count;
    let variance = (sumsq_return_on_premium / count
        - mean_return_on_premium * mean_return_on_premium)
        .max(0.0);
    (mean_return_on_premium, variance.sqrt())
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
            mean_return_on_premium: 0.0,
            return_volatility_on_premium: 0.0,
            median_return_on_premium: 0.0,
            win_rate: 0.0,
            total_pnl_per_contract: 0.0,
        };
    }

    let mut total_pnl = 0.0;
    let mut total_premium = 0.0;
    let mut wins = 0usize;
    let mut sum_return_on_premium = 0.0;
    let mut sumsq_return_on_premium = 0.0;
    let mut returns = Vec::with_capacity(valid_trades);
    for outcome in outcomes {
        total_pnl += outcome.pnl_per_contract;
        total_premium += outcome.premium_paid;
        if outcome.pnl_per_contract > 0.0 {
            wins += 1;
        }
        let return_on_premium = outcome.return_on_premium;
        sum_return_on_premium += return_on_premium;
        sumsq_return_on_premium += return_on_premium * return_on_premium;
        returns.push(return_on_premium);
    }
    let (mean_return_on_premium, return_volatility_on_premium) =
        return_on_premium_mean_and_volatility(
            sum_return_on_premium,
            sumsq_return_on_premium,
            valid_trades,
        );

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
        mean_return_on_premium,
        return_volatility_on_premium,
        median_return_on_premium: median(returns),
        win_rate: wins as f64 / valid_trades as f64,
        total_pnl_per_contract: total_pnl,
    }
}

#[derive(Clone, Debug)]
struct CandidateAxis {
    structure_family: OptionsStructureFamily,
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
            structure_family: req.structure_family,
            delay_minutes: req.delay_minutes.clone(),
            dte_days: req.dte_buckets.clone(),
            moneyness_buckets: req.moneyness_buckets.clone(),
        })
    }

    fn slot_count_per_event(&self) -> usize {
        self.delay_minutes.len() * self.dte_days.len() * self.moneyness_buckets.len()
    }

    fn local_slot_offset(
        &self,
        delay_index: usize,
        dte_index: usize,
        moneyness_index: usize,
    ) -> usize {
        let delay_stride = self.dte_days.len() * self.moneyness_buckets.len();
        let dte_stride = self.moneyness_buckets.len();
        delay_index * delay_stride + dte_index * dte_stride + moneyness_index
    }

    fn slot_index(
        &self,
        event_index: usize,
        delay_index: usize,
        dte_index: usize,
        moneyness_index: usize,
    ) -> usize {
        let per_event = self.slot_count_per_event();
        event_index * per_event + self.local_slot_offset(delay_index, dte_index, moneyness_index)
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

    fn invalid_reason_counts_for_slots<I>(
        &self,
        slot_indexes: I,
    ) -> Vec<OptionsCandidateInvalidReasonCount>
    where
        I: IntoIterator<Item = usize>,
    {
        let mut counts: BTreeMap<OptionsCandidateInvalidReason, usize> = BTreeMap::new();
        for slot_index in slot_indexes {
            if let Some(reason) = self.invalid_reasons.get(slot_index).copied().flatten() {
                *counts.entry(reason).or_default() += 1;
            }
        }
        counts
            .into_iter()
            .map(|(reason, count)| OptionsCandidateInvalidReasonCount { reason, count })
            .collect()
    }

    fn axis_coverage(&self) -> OptionsCandidateAxisCoverage {
        let per_event = self.axis.slot_count_per_event();
        if per_event == 0 {
            return OptionsCandidateAxisCoverage::default();
        }
        let event_count = self.slots.len() / per_event;
        let delay_len = self.axis.delay_minutes.len();
        let dte_len = self.axis.dte_days.len();
        let moneyness_len = self.axis.moneyness_buckets.len();

        let delay_minutes = self
            .axis
            .delay_minutes
            .iter()
            .copied()
            .enumerate()
            .map(|(delay_index, delay_minutes)| {
                let mut valid_slot_count = 0usize;
                let mut invalid_slot_indexes = Vec::new();
                for event_index in 0..event_count {
                    for dte_index in 0..dte_len {
                        for moneyness_index in 0..moneyness_len {
                            let slot_index = self.axis.slot_index(
                                event_index,
                                delay_index,
                                dte_index,
                                moneyness_index,
                            );
                            if self.slots.get(slot_index).copied().flatten().is_some() {
                                valid_slot_count += 1;
                            } else {
                                invalid_slot_indexes.push(slot_index);
                            }
                        }
                    }
                }
                let slot_count = event_count * dte_len * moneyness_len;
                OptionsDelayBucketCoverage {
                    delay_minutes,
                    slot_count,
                    valid_slot_count,
                    invalid_slot_count: slot_count.saturating_sub(valid_slot_count),
                    valid_rate: ratio_or_zero(valid_slot_count, slot_count),
                    invalid_reasons: self.invalid_reason_counts_for_slots(invalid_slot_indexes),
                }
            })
            .collect();

        let dte_days = self
            .axis
            .dte_days
            .iter()
            .copied()
            .enumerate()
            .map(|(dte_index, dte_days)| {
                let mut valid_slot_count = 0usize;
                let mut invalid_slot_indexes = Vec::new();
                for event_index in 0..event_count {
                    for delay_index in 0..delay_len {
                        for moneyness_index in 0..moneyness_len {
                            let slot_index = self.axis.slot_index(
                                event_index,
                                delay_index,
                                dte_index,
                                moneyness_index,
                            );
                            if self.slots.get(slot_index).copied().flatten().is_some() {
                                valid_slot_count += 1;
                            } else {
                                invalid_slot_indexes.push(slot_index);
                            }
                        }
                    }
                }
                let slot_count = event_count * delay_len * moneyness_len;
                OptionsDteBucketCoverage {
                    dte_days,
                    slot_count,
                    valid_slot_count,
                    invalid_slot_count: slot_count.saturating_sub(valid_slot_count),
                    valid_rate: ratio_or_zero(valid_slot_count, slot_count),
                    invalid_reasons: self.invalid_reason_counts_for_slots(invalid_slot_indexes),
                }
            })
            .collect();

        let moneyness_buckets = self
            .axis
            .moneyness_buckets
            .iter()
            .copied()
            .enumerate()
            .map(|(moneyness_index, moneyness_bucket)| {
                let mut valid_slot_count = 0usize;
                let mut invalid_slot_indexes = Vec::new();
                for event_index in 0..event_count {
                    for delay_index in 0..delay_len {
                        for dte_index in 0..dte_len {
                            let slot_index = self.axis.slot_index(
                                event_index,
                                delay_index,
                                dte_index,
                                moneyness_index,
                            );
                            if self.slots.get(slot_index).copied().flatten().is_some() {
                                valid_slot_count += 1;
                            } else {
                                invalid_slot_indexes.push(slot_index);
                            }
                        }
                    }
                }
                let slot_count = event_count * delay_len * dte_len;
                OptionsMoneynessBucketCoverage {
                    moneyness_bucket,
                    slot_count,
                    valid_slot_count,
                    invalid_slot_count: slot_count.saturating_sub(valid_slot_count),
                    valid_rate: ratio_or_zero(valid_slot_count, slot_count),
                    invalid_reasons: self.invalid_reason_counts_for_slots(invalid_slot_indexes),
                }
            })
            .collect();

        OptionsCandidateAxisCoverage {
            delay_minutes,
            dte_days,
            moneyness_buckets,
        }
    }

    fn invalid_reason(
        &self,
        event_index: usize,
        delay_index: usize,
        dte_index: usize,
        moneyness_index: usize,
    ) -> Option<OptionsCandidateInvalidReason> {
        let slot_index = self
            .axis
            .slot_index(event_index, delay_index, dte_index, moneyness_index);
        self.invalid_reasons.get(slot_index).copied().flatten()
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

    fn local_summary_offset(
        &self,
        hold_index: usize,
        target_index: usize,
        stop_index: usize,
    ) -> usize {
        let hold_stride = self.target_values.len() * self.stop_values.len();
        let target_stride = self.stop_values.len();
        hold_index * hold_stride + target_index * target_stride + stop_index
    }

    fn summary_index(
        &self,
        candidate_slot_index: usize,
        hold_index: usize,
        target_index: usize,
        stop_index: usize,
    ) -> usize {
        let per_candidate = self.summary_count_per_candidate();
        candidate_slot_index * per_candidate
            + self.local_summary_offset(hold_index, target_index, stop_index)
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
                                    structure_family: candidate_axis.structure_family,
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
    retained_summary_state: RetainedSummaryState,
}

#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RetainedMetricState {
    Missing,
    Exact,
    AggregateNeedsEnrichment,
}

#[derive(Clone, Copy, Debug)]
struct RetainedSummaryState {
    selection: RetainedMetricState,
    holdout: RetainedMetricState,
    stress: RetainedMetricState,
    harsher: RetainedMetricState,
}

impl RetainedSummaryState {
    fn cpu_initial(holdout_events: usize) -> Self {
        let holdout_state = if holdout_events > 0 {
            RetainedMetricState::Exact
        } else {
            RetainedMetricState::Missing
        };
        Self {
            selection: RetainedMetricState::Exact,
            holdout: holdout_state,
            stress: holdout_state,
            harsher: RetainedMetricState::Missing,
        }
    }

    #[cfg_attr(
        not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
        allow(dead_code)
    )]
    fn gpu_initial(holdout_events: usize) -> Self {
        let holdout_state = if holdout_events > 0 {
            RetainedMetricState::AggregateNeedsEnrichment
        } else {
            RetainedMetricState::Missing
        };
        Self {
            selection: RetainedMetricState::AggregateNeedsEnrichment,
            holdout: holdout_state,
            stress: holdout_state,
            harsher: holdout_state,
        }
    }
}

#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
#[derive(Clone, Copy, Debug, Default)]
struct ExactifyTimingBreakdown {
    total_us: u64,
    summary_overwrite_us: u64,
    enrichment_us: u64,
    extreme_us: u64,
}

#[cfg_attr(
    not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
    allow(dead_code)
)]
#[derive(Clone, Copy, Debug, Default)]
struct SealedStageTimingBreakdown {
    total_us: u64,
    non_extreme_us: u64,
    extreme_us: u64,
}

#[derive(Clone, Debug)]
struct PreparedOptionsExpressionRun {
    events: Vec<OptionsSignalEvent>,
    compiled_tuples: Vec<CompiledTupleSpec>,
    baseline_candidate_cache: StructuralCandidateCache,
    baseline_exit_summary_cache: ExitSummaryCache,
    conservative_candidate_cache: StructuralCandidateCache,
    conservative_exit_summary_cache: ExitSummaryCache,
    harsher_candidate_cache: StructuralCandidateCache,
    harsher_exit_summary_cache: ExitSummaryCache,
    diagnostics: OptionsExpressionDiagnostics,
    overlap_window: OptionsExpressionOverlapWindowSummary,
    staging: OptionsExpressionStagingStats,
    timings: OptionsExpressionFeasibilityTimingStats,
    cache_stats: OptionsExpressionFeasibilityCacheStats,
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    baseline_gpu_inputs: GpuFlatScoreInputs,
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    conservative_gpu_inputs: GpuFlatScoreInputs,
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    harsher_gpu_inputs: GpuFlatScoreInputs,
}

#[derive(Clone, Debug)]
struct RankedTuplePass {
    ranked: Vec<RankedCompiledTuple>,
    backend_used: &'static str,
    backend_fallback_reason: Option<String>,
}

fn structural_candidate_counts(prepared: &PreparedOptionsExpressionRun) -> (usize, usize, usize) {
    let candidate_slot_count = prepared.baseline_candidate_cache.slots.len();
    let compiled_candidate_count = prepared
        .baseline_candidate_cache
        .slots
        .iter()
        .filter(|entry| entry.is_some())
        .count();
    let invalid_slot_count = prepared
        .baseline_candidate_cache
        .invalid_reasons
        .iter()
        .filter(|entry| entry.is_some())
        .count();
    (
        candidate_slot_count,
        compiled_candidate_count,
        invalid_slot_count,
    )
}

fn projected_event_tuple_evaluations(prepared: &PreparedOptionsExpressionRun) -> u64 {
    (prepared.compiled_tuples.len() as u64).saturating_mul(prepared.events.len() as u64)
}

fn measured_best_available_backend(
    cpu_ranking_us: u64,
    gpu_ranking_us: Option<u64>,
) -> &'static str {
    if let Some(gpu_us) = gpu_ranking_us {
        if gpu_us > 0 && gpu_us < cpu_ranking_us {
            return "gpu_compiled_tensor_hybrid";
        }
    }
    "cpu_reference"
}

fn evaluate_auto_backend_decision(
    prepared: &PreparedOptionsExpressionRun,
    gpu_available: bool,
    measured_gpu_speedup_vs_cpu: Option<f64>,
) -> OptionsExpressionAutoBackendDecision {
    let (candidate_slot_count, compiled_candidate_count, invalid_slot_count) =
        structural_candidate_counts(prepared);
    let candidate_density = ratio_or_zero(compiled_candidate_count, candidate_slot_count);
    let invalid_slot_rate = ratio_or_zero(invalid_slot_count, candidate_slot_count);
    let projected_event_tuple_evals = projected_event_tuple_evaluations(prepared);
    let mut reasons = Vec::new();

    if !gpu_available {
        reasons.push("CUDA device unavailable for auto backend selection.".to_string());
    }
    if candidate_density < OPTIONS_AUTO_GPU_MIN_CANDIDATE_DENSITY {
        reasons.push(format!(
            "Candidate density {:.1}% is below the current auto-GPU floor of {:.1}%.",
            candidate_density * 100.0,
            OPTIONS_AUTO_GPU_MIN_CANDIDATE_DENSITY * 100.0
        ));
    }
    if invalid_slot_rate > OPTIONS_AUTO_GPU_MAX_INVALID_SLOT_RATE {
        reasons.push(format!(
            "Invalid-slot rate {:.1}% exceeds the current auto-GPU ceiling of {:.1}%.",
            invalid_slot_rate * 100.0,
            OPTIONS_AUTO_GPU_MAX_INVALID_SLOT_RATE * 100.0
        ));
    }
    if prepared.compiled_tuples.len() < OPTIONS_AUTO_GPU_MIN_COMPILED_TUPLES {
        reasons.push(format!(
            "Compiled tuple count {} is below the current auto-GPU floor of {}.",
            prepared.compiled_tuples.len(),
            OPTIONS_AUTO_GPU_MIN_COMPILED_TUPLES
        ));
    }
    if prepared.diagnostics.search_events < OPTIONS_AUTO_GPU_MIN_SEARCH_EVENTS {
        reasons.push(format!(
            "Search-event count {} is below the current auto-GPU floor of {}.",
            prepared.diagnostics.search_events, OPTIONS_AUTO_GPU_MIN_SEARCH_EVENTS
        ));
    }
    if projected_event_tuple_evals < OPTIONS_AUTO_GPU_MIN_PROJECTED_EVENT_TUPLES {
        reasons.push(format!(
            "Projected event-tuple work {} is below the current auto-GPU floor of {}.",
            projected_event_tuple_evals, OPTIONS_AUTO_GPU_MIN_PROJECTED_EVENT_TUPLES
        ));
    }
    if let Some(speedup) = measured_gpu_speedup_vs_cpu {
        if speedup < OPTIONS_AUTO_GPU_MIN_MEASURED_SPEEDUP {
            reasons.push(format!(
                "Measured GPU hybrid speedup {:.2}x is below the current promotion gate of {:.1}x.",
                speedup, OPTIONS_AUTO_GPU_MIN_MEASURED_SPEEDUP
            ));
        }
    }

    let gpu_eligible = reasons.is_empty();
    OptionsExpressionAutoBackendDecision {
        recommended_backend: if gpu_eligible {
            "gpu_compiled_tensor_hybrid".to_string()
        } else {
            "cpu_reference".to_string()
        },
        gpu_eligible,
        reasons,
    }
}

fn runtime_auto_backend_decision(
    prepared: &PreparedOptionsExpressionRun,
    gpu_available: bool,
) -> OptionsExpressionAutoBackendDecision {
    evaluate_auto_backend_decision(prepared, gpu_available, None)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn options_gpu_combined_scoring_enabled() -> bool {
    std::env::var("VECTORGRID_OPTIONS_GPU_DISABLE_COMBINED_SCORING")
        .ok()
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            !(normalized == "1" || normalized == "true" || normalized == "yes")
        })
        .unwrap_or(true)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn push_unique_gpu_note(target: &mut Vec<String>, note: impl Into<String>) {
    let note = note.into();
    if !target.iter().any(|existing| existing == &note) {
        target.push(note);
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn accumulate_cache_usage(
    target: &mut OptionsGpuRuntimeCacheUsage,
    usage: OptionsGpuRuntimeCacheUsage,
) {
    target.shared_input_cache_hits += usage.shared_input_cache_hits;
    target.shared_input_cache_misses += usage.shared_input_cache_misses;
    target.outcome_input_cache_hits += usage.outcome_input_cache_hits;
    target.outcome_input_cache_misses += usage.outcome_input_cache_misses;
    target.output_buffer_reuses += usage.output_buffer_reuses;
    target.output_buffer_allocations += usage.output_buffer_allocations;
    target.kernel_launch_count += usage.kernel_launch_count;
    target.tuple_chunk_count = target.tuple_chunk_count.max(usage.tuple_chunk_count);
    target.tuple_chunk_size = target.tuple_chunk_size.max(usage.tuple_chunk_size);
    target.gpu_tuple_budget_bytes = target
        .gpu_tuple_budget_bytes
        .max(usage.gpu_tuple_budget_bytes);
    target.gpu_outcome_tensor_bytes = target
        .gpu_outcome_tensor_bytes
        .max(usage.gpu_outcome_tensor_bytes);
    target.gpu_shared_input_bytes_per_chunk = target
        .gpu_shared_input_bytes_per_chunk
        .max(usage.gpu_shared_input_bytes_per_chunk);
    target.gpu_output_bytes_per_chunk = target
        .gpu_output_bytes_per_chunk
        .max(usage.gpu_output_bytes_per_chunk);
    target.gpu_total_device_bytes_per_chunk = target
        .gpu_total_device_bytes_per_chunk
        .max(usage.gpu_total_device_bytes_per_chunk);
    target.gpu_device_total_memory_bytes = target
        .gpu_device_total_memory_bytes
        .max(usage.gpu_device_total_memory_bytes);
    target.gpu_device_free_memory_bytes = target
        .gpu_device_free_memory_bytes
        .max(usage.gpu_device_free_memory_bytes);
    target.gpu_device_usable_memory_bytes = target
        .gpu_device_usable_memory_bytes
        .max(usage.gpu_device_usable_memory_bytes);
    if target.gpu_memory_budget_source.is_empty() && !usage.gpu_memory_budget_source.is_empty() {
        target.gpu_memory_budget_source = usage.gpu_memory_budget_source.clone();
    }
    target.gpu_outcome_tensor_fits_in_memory |= usage.gpu_outcome_tensor_fits_in_memory;
    target.gpu_chunk_plan_fits_in_memory |= usage.gpu_chunk_plan_fits_in_memory;
    for note in usage.gpu_plan_notes {
        push_unique_gpu_note(&mut target.gpu_plan_notes, note);
    }
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
#[derive(Clone, Copy, Debug, Default)]
struct TupleAggregateMetrics {
    valid_trades: usize,
    invalid_events: usize,
    win_count: usize,
    total_pnl_per_contract: f64,
    total_premium_paid: f64,
    sum_return_on_premium: f64,
    sumsq_return_on_premium: f64,
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
fn metrics_from_aggregate(aggregate: TupleAggregateMetrics) -> OptionsExpressionMetrics {
    let event_count = aggregate.valid_trades + aggregate.invalid_events;
    if event_count == 0 {
        return OptionsExpressionMetrics::default();
    }
    if aggregate.valid_trades == 0 {
        return OptionsExpressionMetrics {
            valid_trades: 0,
            invalid_events: aggregate.invalid_events,
            invalid_rate: aggregate.invalid_events as f64 / event_count as f64,
            expectancy_per_trade: 0.0,
            expectancy_per_premium: 0.0,
            mean_return_on_premium: 0.0,
            return_volatility_on_premium: 0.0,
            median_return_on_premium: 0.0,
            win_rate: 0.0,
            total_pnl_per_contract: 0.0,
        };
    }
    let (mean_return_on_premium, return_volatility_on_premium) =
        return_on_premium_mean_and_volatility(
            aggregate.sum_return_on_premium,
            aggregate.sumsq_return_on_premium,
            aggregate.valid_trades,
        );
    OptionsExpressionMetrics {
        valid_trades: aggregate.valid_trades,
        invalid_events: aggregate.invalid_events,
        invalid_rate: aggregate.invalid_events as f64 / event_count as f64,
        expectancy_per_trade: aggregate.total_pnl_per_contract / aggregate.valid_trades as f64,
        expectancy_per_premium: if aggregate.total_premium_paid > 0.0 {
            aggregate.total_pnl_per_contract / aggregate.total_premium_paid
        } else {
            0.0
        },
        mean_return_on_premium,
        return_volatility_on_premium,
        median_return_on_premium: 0.0,
        win_rate: aggregate.win_count as f64 / aggregate.valid_trades as f64,
        total_pnl_per_contract: aggregate.total_pnl_per_contract,
    }
}

fn prepare_options_expression_run(
    req: &OptionsExpressionRunRequest,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<PreparedOptionsExpressionRun, String> {
    let _nvtx_scope = crate::nvtx::scope("engine", "options_prepare");
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
        req.split_assignment_mode,
    )?;
    let candidate_axis = CandidateAxis::from_request(req)?;
    let exit_axis = ExitAxis::from_request(req)?;
    let (compiled_tuples, tuple_compile_ms) = {
        let _tuple_compile_scope = crate::nvtx::scope("engine", "options_tuple_compile");
        let tuple_compile_start = Instant::now();
        let compiled_tuples = compile_tuple_grid(&candidate_axis, &exit_axis);
        (compiled_tuples, elapsed_ms(tuple_compile_start))
    };
    let mut cache_stats = OptionsExpressionFeasibilityCacheStats::default();
    let baseline_candidate_key = structural_candidate_cache_key(
        signal_manifest,
        dataset,
        &candidate_axis,
        req.fill_model,
        &req.filters,
        AggregateStressMode::Baseline,
    );
    let (baseline_candidate_lookup, baseline_candidate_compile_ms) = {
        let _baseline_candidate_scope =
            crate::nvtx::scope("engine", "options_prepare_baseline_candidates");
        let baseline_candidate_start = Instant::now();
        let baseline_candidate_lookup = get_or_build_structural_candidate_cache(
            signal_manifest,
            &events,
            dataset,
            candidate_axis.clone(),
            req.fill_model,
            &req.filters,
            AggregateStressMode::Baseline,
            &mut cache_stats,
        );
        let baseline_candidate_compile_ms = if baseline_candidate_lookup.hit {
            0
        } else {
            elapsed_ms(baseline_candidate_start)
        };
        (baseline_candidate_lookup, baseline_candidate_compile_ms)
    };
    let baseline_candidate_cache = baseline_candidate_lookup.value;
    let baseline_exit_summary_start = Instant::now();
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let baseline_exit_summary_key = exit_summary_cache_key(&baseline_candidate_key, &exit_axis);
    let baseline_exit_summary_lookup = get_or_build_exit_summary_cache(
        &baseline_candidate_key,
        &events,
        dataset,
        &baseline_candidate_cache,
        exit_axis.clone(),
        AggregateStressMode::Baseline,
        &mut cache_stats,
    );
    let baseline_exit_summary_compile_ms = if baseline_exit_summary_lookup.hit {
        0
    } else {
        elapsed_ms(baseline_exit_summary_start)
    };
    let baseline_exit_summary_cache = baseline_exit_summary_lookup.value;
    let conservative_candidate_key = structural_candidate_cache_key(
        signal_manifest,
        dataset,
        &candidate_axis,
        req.fill_model,
        &req.filters,
        AggregateStressMode::Conservative,
    );
    let (conservative_candidate_lookup, conservative_candidate_compile_ms) = {
        let _conservative_candidate_scope =
            crate::nvtx::scope("engine", "options_prepare_conservative_candidates");
        let conservative_candidate_start = Instant::now();
        let conservative_candidate_lookup = get_or_build_structural_candidate_cache(
            signal_manifest,
            &events,
            dataset,
            candidate_axis.clone(),
            req.fill_model,
            &req.filters,
            AggregateStressMode::Conservative,
            &mut cache_stats,
        );
        let conservative_candidate_compile_ms = if conservative_candidate_lookup.hit {
            0
        } else {
            elapsed_ms(conservative_candidate_start)
        };
        (
            conservative_candidate_lookup,
            conservative_candidate_compile_ms,
        )
    };
    let conservative_candidate_cache = conservative_candidate_lookup.value;
    let conservative_exit_summary_start = Instant::now();
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let conservative_exit_summary_key =
        exit_summary_cache_key(&conservative_candidate_key, &exit_axis);
    let conservative_exit_summary_lookup = get_or_build_exit_summary_cache(
        &conservative_candidate_key,
        &events,
        dataset,
        &conservative_candidate_cache,
        exit_axis.clone(),
        AggregateStressMode::Conservative,
        &mut cache_stats,
    );
    let conservative_exit_summary_compile_ms = if conservative_exit_summary_lookup.hit {
        0
    } else {
        elapsed_ms(conservative_exit_summary_start)
    };
    let conservative_exit_summary_cache = conservative_exit_summary_lookup.value;
    let harsher_candidate_key = structural_candidate_cache_key(
        signal_manifest,
        dataset,
        &candidate_axis,
        req.fill_model,
        &req.filters,
        AggregateStressMode::Harsher,
    );
    let (harsher_candidate_lookup, harsher_candidate_compile_ms) = {
        let _harsher_candidate_scope =
            crate::nvtx::scope("engine", "options_prepare_harsher_candidates");
        let harsher_candidate_start = Instant::now();
        let harsher_candidate_lookup = get_or_build_structural_candidate_cache(
            signal_manifest,
            &events,
            dataset,
            candidate_axis,
            req.fill_model,
            &req.filters,
            AggregateStressMode::Harsher,
            &mut cache_stats,
        );
        let harsher_candidate_compile_ms = if harsher_candidate_lookup.hit {
            0
        } else {
            elapsed_ms(harsher_candidate_start)
        };
        (harsher_candidate_lookup, harsher_candidate_compile_ms)
    };
    let harsher_candidate_cache = harsher_candidate_lookup.value;
    let harsher_exit_summary_start = Instant::now();
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let harsher_exit_summary_key = exit_summary_cache_key(&harsher_candidate_key, &exit_axis);
    let harsher_exit_summary_lookup = get_or_build_exit_summary_cache(
        &harsher_candidate_key,
        &events,
        dataset,
        &harsher_candidate_cache,
        exit_axis,
        AggregateStressMode::Harsher,
        &mut cache_stats,
    );
    let harsher_exit_summary_compile_ms = if harsher_exit_summary_lookup.hit {
        0
    } else {
        elapsed_ms(harsher_exit_summary_start)
    };
    let harsher_exit_summary_cache = harsher_exit_summary_lookup.value;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let baseline_gpu_inputs = build_gpu_flat_score_inputs(
        &compiled_tuples,
        &baseline_candidate_cache,
        &baseline_exit_summary_cache,
        &baseline_exit_summary_key,
    )?;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let conservative_gpu_inputs = build_gpu_flat_score_inputs(
        &compiled_tuples,
        &conservative_candidate_cache,
        &conservative_exit_summary_cache,
        &conservative_exit_summary_key,
    )?;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let harsher_gpu_inputs = build_gpu_flat_score_inputs(
        &compiled_tuples,
        &harsher_candidate_cache,
        &harsher_exit_summary_cache,
        &harsher_exit_summary_key,
    )?;
    let staging = OptionsExpressionStagingStats {
        candidate_slot_count: baseline_candidate_cache.slots.len(),
        compiled_candidate_count: baseline_candidate_cache
            .slots
            .iter()
            .filter(|entry| entry.is_some())
            .count(),
        baseline_exit_summary_count: baseline_exit_summary_cache.outcomes.len(),
        conservative_exit_summary_count: conservative_exit_summary_cache.outcomes.len(),
        harsher_exit_summary_count: harsher_exit_summary_cache.outcomes.len(),
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
    let overlap_window = build_overlap_window_summary(&events, &diagnostics, dataset);
    let timings = OptionsExpressionFeasibilityTimingStats {
        prepare_ms: 0,
        prepare_us: 0,
        tuple_compile_ms,
        baseline_candidate_compile_ms,
        baseline_exit_summary_compile_ms,
        conservative_candidate_compile_ms,
        conservative_exit_summary_compile_ms,
        harsher_candidate_compile_ms,
        harsher_exit_summary_compile_ms,
        gpu_device_resolution_ms: None,
        gpu_device_resolution_us: None,
        cpu_ranking_ms: 0,
        cpu_ranking_us: 0,
        gpu_plan_probe_ms: None,
        gpu_plan_probe_us: None,
        gpu_ranking_ms: None,
        gpu_ranking_us: None,
        gpu_tensor_score_ms: None,
        gpu_tensor_score_us: None,
        gpu_exactify_ms: None,
        gpu_exactify_us: None,
        gpu_exactify_summary_overwrite_ms: None,
        gpu_exactify_summary_overwrite_us: None,
        gpu_exactify_enrichment_ms: None,
        gpu_exactify_enrichment_us: None,
        gpu_exactify_extreme_ms: None,
        gpu_exactify_extreme_us: None,
        gpu_sealed_stage_ms: None,
        gpu_sealed_stage_us: None,
        gpu_sealed_non_extreme_ms: None,
        gpu_sealed_non_extreme_us: None,
        gpu_sealed_extreme_ms: None,
        gpu_sealed_extreme_us: None,
        report_finalize_ms: 0,
        report_finalize_us: 0,
        gpu_speedup_vs_cpu: None,
    };

    Ok(PreparedOptionsExpressionRun {
        events,
        compiled_tuples,
        baseline_candidate_cache,
        baseline_exit_summary_cache,
        conservative_candidate_cache,
        conservative_exit_summary_cache,
        harsher_candidate_cache,
        harsher_exit_summary_cache,
        diagnostics,
        overlap_window,
        staging,
        timings,
        cache_stats,
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        baseline_gpu_inputs,
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        conservative_gpu_inputs,
        #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
        harsher_gpu_inputs,
    })
}

fn sort_ranked_results(ranked_results: &mut [RankedCompiledTuple]) {
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
}

fn compare_optional_metrics_desc(
    left: Option<&OptionsExpressionMetrics>,
    right: Option<&OptionsExpressionMetrics>,
) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right
            .expectancy_per_premium
            .partial_cmp(&left.expectancy_per_premium)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                right
                    .expectancy_per_trade
                    .partial_cmp(&left.expectancy_per_trade)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                right
                    .win_rate
                    .partial_cmp(&left.win_rate)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| right.valid_trades.cmp(&left.valid_trades)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn resequence_pre_sealed_ranks(ranked_results: &mut [RankedCompiledTuple]) {
    for (index, row) in ranked_results.iter_mut().enumerate() {
        row.result.pre_sealed_rank = index + 1;
    }
}

fn assign_sealed_final_ranks(ranked_results: &mut [RankedCompiledTuple]) {
    let mut finalist_positions: Vec<usize> = ranked_results
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.result.sealed_finalist.then_some(index))
        .collect();
    finalist_positions.sort_by(|&left_index, &right_index| {
        let left = &ranked_results[left_index].result;
        let right = &ranked_results[right_index].result;
        compare_optional_metrics_desc(
            left.sealed_evaluation_metrics.as_ref(),
            right.sealed_evaluation_metrics.as_ref(),
        )
        .then_with(|| {
            compare_optional_metrics_desc(
                left.sealed_stress_metrics.as_ref(),
                right.sealed_stress_metrics.as_ref(),
            )
        })
        .then_with(|| {
            compare_optional_metrics_desc(
                left.sealed_harsher_stress_metrics.as_ref(),
                right.sealed_harsher_stress_metrics.as_ref(),
            )
        })
        .then_with(|| {
            compare_optional_metrics_desc(
                left.sealed_extreme_stress_metrics.as_ref(),
                right.sealed_extreme_stress_metrics.as_ref(),
            )
        })
        .then_with(|| {
            right
                .robust_score
                .partial_cmp(&left.robust_score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.pre_sealed_rank.cmp(&right.pre_sealed_rank))
    });
    for row in ranked_results.iter_mut() {
        row.result.sealed_final_rank = 0;
    }
    for (sealed_rank, index) in finalist_positions.into_iter().enumerate() {
        ranked_results[index].result.sealed_final_rank = sealed_rank + 1;
    }
}

fn refresh_validation_verdicts(ranked_results: &mut [RankedCompiledTuple]) {
    for ranked in ranked_results.iter_mut() {
        ranked.result.validation_verdict = ranked.result.inferred_validation_verdict();
    }
}

fn recompute_retained_robust_scores(ranked_results: &mut [RankedCompiledTuple]) {
    for ranked in ranked_results.iter_mut() {
        ranked.result.robust_score = robust_score(
            &ranked.result.selection_metrics,
            ranked.result.holdout_metrics.as_ref(),
            None,
            ranked.result.stress_metrics.as_ref(),
            ranked.result.harsher_stress_metrics.as_ref(),
            ranked.result.extreme_stress_metrics.as_ref(),
        );
    }
}

fn exactify_retained_summary_metrics(
    prepared: &PreparedOptionsExpressionRun,
    ranked_results: &mut [RankedCompiledTuple],
) {
    let holdout_events = prepared.diagnostics.holdout_events;
    for ranked in ranked_results.iter_mut() {
        if matches!(
            ranked.retained_summary_state.selection,
            RetainedMetricState::Missing
        ) {
            ranked.result.selection_metrics = evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::Search,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            );
            ranked.retained_summary_state.selection = RetainedMetricState::Exact;
        }
        if holdout_events == 0 {
            ranked.retained_summary_state.holdout = RetainedMetricState::Missing;
            ranked.retained_summary_state.stress = RetainedMetricState::Missing;
            ranked.retained_summary_state.harsher = RetainedMetricState::Missing;
            ranked.result.holdout_metrics = None;
            ranked.result.stress_metrics = None;
            ranked.result.harsher_stress_metrics = None;
            continue;
        }
        if matches!(
            ranked.retained_summary_state.holdout,
            RetainedMetricState::Missing
        ) {
            ranked.result.holdout_metrics = Some(evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            ));
            ranked.retained_summary_state.holdout = RetainedMetricState::Exact;
        }
        if matches!(
            ranked.retained_summary_state.stress,
            RetainedMetricState::Missing
        ) {
            ranked.result.stress_metrics = Some(evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.conservative_candidate_cache,
                &prepared.conservative_exit_summary_cache,
            ));
            ranked.retained_summary_state.stress = RetainedMetricState::Exact;
        }
        if matches!(
            ranked.retained_summary_state.harsher,
            RetainedMetricState::Missing
        ) {
            ranked.result.harsher_stress_metrics = Some(evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.harsher_candidate_cache,
                &prepared.harsher_exit_summary_cache,
            ));
            ranked.retained_summary_state.harsher = RetainedMetricState::Exact;
        }
    }
}

fn exactify_retained_summary_enrichment(
    prepared: &PreparedOptionsExpressionRun,
    ranked_results: &mut [RankedCompiledTuple],
) {
    let holdout_events = prepared.diagnostics.holdout_events;
    for ranked in ranked_results.iter_mut() {
        if matches!(
            ranked.retained_summary_state.selection,
            RetainedMetricState::AggregateNeedsEnrichment
        ) {
            ranked.result.selection_metrics.median_return_on_premium =
                evaluate_median_return_on_premium_for_split(
                    &ranked.compiled,
                    &prepared.events,
                    OptionsEvaluationSplit::Search,
                    &prepared.baseline_candidate_cache,
                    &prepared.baseline_exit_summary_cache,
                );
            ranked.retained_summary_state.selection = RetainedMetricState::Exact;
        }
        if holdout_events == 0 {
            continue;
        }
        if matches!(
            ranked.retained_summary_state.holdout,
            RetainedMetricState::AggregateNeedsEnrichment
        ) {
            if let Some(metrics) = ranked.result.holdout_metrics.as_mut() {
                metrics.median_return_on_premium = evaluate_median_return_on_premium_for_split(
                    &ranked.compiled,
                    &prepared.events,
                    OptionsEvaluationSplit::Holdout,
                    &prepared.baseline_candidate_cache,
                    &prepared.baseline_exit_summary_cache,
                );
            }
            ranked.retained_summary_state.holdout = RetainedMetricState::Exact;
        }
        if matches!(
            ranked.retained_summary_state.stress,
            RetainedMetricState::AggregateNeedsEnrichment
        ) {
            if let Some(metrics) = ranked.result.stress_metrics.as_mut() {
                metrics.median_return_on_premium = evaluate_median_return_on_premium_for_split(
                    &ranked.compiled,
                    &prepared.events,
                    OptionsEvaluationSplit::Holdout,
                    &prepared.conservative_candidate_cache,
                    &prepared.conservative_exit_summary_cache,
                );
            }
            ranked.retained_summary_state.stress = RetainedMetricState::Exact;
        }
        if matches!(
            ranked.retained_summary_state.harsher,
            RetainedMetricState::AggregateNeedsEnrichment
        ) {
            if let Some(metrics) = ranked.result.harsher_stress_metrics.as_mut() {
                metrics.median_return_on_premium = evaluate_median_return_on_premium_for_split(
                    &ranked.compiled,
                    &prepared.events,
                    OptionsEvaluationSplit::Holdout,
                    &prepared.harsher_candidate_cache,
                    &prepared.harsher_exit_summary_cache,
                );
            }
            ranked.retained_summary_state.harsher = RetainedMetricState::Exact;
        }
    }
}

fn exactify_retained_extreme_stress_metrics(
    prepared: &PreparedOptionsExpressionRun,
    dataset: &ImportedOptionsDataset,
    ranked_results: &mut [RankedCompiledTuple],
) {
    let holdout_events = prepared.diagnostics.holdout_events;
    let index = IndexedOptionsDataset::from_dataset(dataset, &dataset.manifest.root_symbol);
    for ranked in ranked_results.iter_mut() {
        ranked.result.extreme_stress_metrics = if holdout_events > 0 {
            Some(evaluate_exact_stress_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &index,
                &prepared.baseline_candidate_cache,
                dataset.manifest.price_semantics,
                AggregateStressMode::Extreme,
            ))
        } else {
            None
        };
    }
}

fn exactify_ranked_results(
    prepared: &PreparedOptionsExpressionRun,
    dataset: &ImportedOptionsDataset,
    ranked_results: &mut [RankedCompiledTuple],
) -> ExactifyTimingBreakdown {
    let _nvtx_scope = crate::nvtx::scope("engine", "options_exactify_total");
    let total_start = Instant::now();

    let summary_overwrite_us = {
        let _summary_overwrite_scope =
            crate::nvtx::scope("engine", "options_exactify_summary_overwrite");
        let summary_overwrite_start = Instant::now();
        exactify_retained_summary_metrics(prepared, ranked_results);
        elapsed_us(summary_overwrite_start)
    };

    let enrichment_us = {
        let _enrichment_scope = crate::nvtx::scope("engine", "options_exactify_enrichment");
        let enrichment_start = Instant::now();
        exactify_retained_summary_enrichment(prepared, ranked_results);
        elapsed_us(enrichment_start)
    };

    let extreme_us = {
        let _extreme_scope = crate::nvtx::scope("engine", "options_exactify_extreme");
        let extreme_start = Instant::now();
        exactify_retained_extreme_stress_metrics(prepared, dataset, ranked_results);
        elapsed_us(extreme_start)
    };

    recompute_retained_robust_scores(ranked_results);
    refresh_validation_verdicts(ranked_results);

    ExactifyTimingBreakdown {
        total_us: elapsed_us(total_start),
        summary_overwrite_us,
        enrichment_us,
        extreme_us,
    }
}

fn apply_sealed_finalist_stage(
    req: &OptionsExpressionRunRequest,
    prepared: &PreparedOptionsExpressionRun,
    dataset: &ImportedOptionsDataset,
    ranked_results: &mut [RankedCompiledTuple],
) -> SealedStageTimingBreakdown {
    let _nvtx_scope = crate::nvtx::scope("engine", "options_sealed_stage_total");
    let total_start = Instant::now();
    let sealed_test_events = prepared.diagnostics.sealed_test_events;
    let sealed_finalist_count = if sealed_test_events > 0 {
        req.sealed_finalist_count.max(1).min(ranked_results.len())
    } else {
        0
    };
    let non_extreme_us = {
        let _non_extreme_scope = crate::nvtx::scope("engine", "options_sealed_stage_non_extreme");
        let non_extreme_start = Instant::now();
        sort_ranked_results(ranked_results);
        resequence_pre_sealed_ranks(ranked_results);
        for ranked in ranked_results.iter_mut().take(sealed_finalist_count) {
            ranked.result.sealed_finalist = true;
            ranked.result.sealed_evaluation_metrics = Some(evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::SealedTest,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            ));
            ranked.result.sealed_stress_metrics = Some(evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::SealedTest,
                &prepared.conservative_candidate_cache,
                &prepared.conservative_exit_summary_cache,
            ));
            ranked.result.sealed_harsher_stress_metrics = Some(evaluate_compiled_tuple_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::SealedTest,
                &prepared.harsher_candidate_cache,
                &prepared.harsher_exit_summary_cache,
            ));
        }
        elapsed_us(non_extreme_start)
    };

    let extreme_us = {
        let _extreme_scope = crate::nvtx::scope("engine", "options_sealed_stage_extreme");
        let extreme_start = Instant::now();
        let index = IndexedOptionsDataset::from_dataset(dataset, &dataset.manifest.root_symbol);
        for ranked in ranked_results.iter_mut().take(sealed_finalist_count) {
            ranked.result.sealed_extreme_stress_metrics = Some(evaluate_exact_stress_for_split(
                &ranked.compiled,
                &prepared.events,
                OptionsEvaluationSplit::SealedTest,
                &index,
                &prepared.baseline_candidate_cache,
                dataset.manifest.price_semantics,
                AggregateStressMode::Extreme,
            ));
        }
        assign_sealed_final_ranks(ranked_results);
        elapsed_us(extreme_start)
    };

    SealedStageTimingBreakdown {
        total_us: elapsed_us(total_start),
        non_extreme_us,
        extreme_us,
    }
}

fn finalize_ranked_results(
    req: &OptionsExpressionRunRequest,
    prepared: &PreparedOptionsExpressionRun,
    dataset: &ImportedOptionsDataset,
    ranked_results: &mut [RankedCompiledTuple],
) -> (ExactifyTimingBreakdown, SealedStageTimingBreakdown) {
    let exactify = exactify_ranked_results(prepared, dataset, ranked_results);
    let sealed_stage = apply_sealed_finalist_stage(req, prepared, dataset, ranked_results);
    refresh_validation_verdicts(ranked_results);
    (exactify, sealed_stage)
}

#[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
mod gpu_scoring {
    use super::*;
    use std::time::Instant;

    #[derive(Clone, Copy, Debug, Default)]
    pub(super) struct GpuRankingTimingBreakdown {
        pub(super) total_us: u64,
        pub(super) tensor_score_us: u64,
        pub(super) exactify: ExactifyTimingBreakdown,
        pub(super) sealed_stage: SealedStageTimingBreakdown,
    }

    #[derive(Clone, Debug, Default)]
    pub(super) struct GpuRankingExecutionStats {
        pub(super) timing: GpuRankingTimingBreakdown,
        pub(super) cache_usage: OptionsGpuRuntimeCacheUsage,
    }

    pub(super) struct GpuRankedTuplePass {
        pub(super) ranked_results: Vec<RankedCompiledTuple>,
        pub(super) stats: GpuRankingExecutionStats,
    }

    fn split_event_count(
        prepared: &PreparedOptionsExpressionRun,
        split: OptionsEvaluationSplit,
    ) -> usize {
        match split {
            OptionsEvaluationSplit::Search => prepared.diagnostics.search_events,
            OptionsEvaluationSplit::Holdout => prepared.diagnostics.holdout_events,
            OptionsEvaluationSplit::SealedTest => prepared.diagnostics.sealed_test_events,
        }
    }

    fn split_event_range(
        prepared: &PreparedOptionsExpressionRun,
        split: OptionsEvaluationSplit,
    ) -> (usize, usize) {
        let search_end = prepared.diagnostics.search_events;
        let holdout_end = search_end + prepared.diagnostics.holdout_events;
        match split {
            OptionsEvaluationSplit::Search => (0, search_end),
            OptionsEvaluationSplit::Holdout => (search_end, holdout_end),
            OptionsEvaluationSplit::SealedTest => (holdout_end, prepared.events.len()),
        }
    }

    fn aggregate_from_gpu(
        aggregate: OptionsGpuTupleAggregate,
        split_event_count: usize,
    ) -> TupleAggregateMetrics {
        let valid_trades = aggregate.valid_trades as usize;
        TupleAggregateMetrics {
            valid_trades,
            invalid_events: split_event_count.saturating_sub(valid_trades),
            win_count: aggregate.wins as usize,
            total_pnl_per_contract: aggregate.total_pnl as f64,
            total_premium_paid: aggregate.total_premium as f64,
            sum_return_on_premium: aggregate.sum_return_on_premium as f64,
            sumsq_return_on_premium: aggregate.sumsq_return_on_premium as f64,
        }
    }

    fn expectancy_per_premium_from_gpu_aggregate(aggregate: OptionsGpuTupleAggregate) -> f64 {
        let total_premium = aggregate.total_premium as f64;
        if total_premium > 0.0 {
            aggregate.total_pnl as f64 / total_premium
        } else {
            0.0
        }
    }

    fn compare_gpu_shortlist_indices(
        left_index: usize,
        right_index: usize,
        robust_scores: &[f64],
        search_aggregates: &[OptionsGpuTupleAggregate],
    ) -> Ordering {
        robust_scores[right_index]
            .partial_cmp(&robust_scores[left_index])
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                let right_expectancy = expectancy_per_premium_from_gpu_aggregate(
                    search_aggregates
                        .get(right_index)
                        .copied()
                        .unwrap_or_default(),
                );
                let left_expectancy = expectancy_per_premium_from_gpu_aggregate(
                    search_aggregates
                        .get(left_index)
                        .copied()
                        .unwrap_or_default(),
                );
                right_expectancy
                    .partial_cmp(&left_expectancy)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left_index.cmp(&right_index))
    }

    fn build_ranked_result_from_gpu_aggregates(
        compiled_tuple: &CompiledTupleSpec,
        search_aggregate: OptionsGpuTupleAggregate,
        holdout_aggregate: Option<OptionsGpuTupleAggregate>,
        stress_aggregate: Option<OptionsGpuTupleAggregate>,
        harsher_aggregate: Option<OptionsGpuTupleAggregate>,
        search_event_count: usize,
        holdout_event_count: usize,
        holdout_events: usize,
    ) -> RankedCompiledTuple {
        let selection_metrics =
            metrics_from_aggregate(aggregate_from_gpu(search_aggregate, search_event_count));
        let holdout_metrics = holdout_aggregate.map(|aggregate| {
            metrics_from_aggregate(aggregate_from_gpu(aggregate, holdout_event_count))
        });
        let stress_metrics = stress_aggregate.map(|aggregate| {
            metrics_from_aggregate(aggregate_from_gpu(aggregate, holdout_event_count))
        });
        let harsher_stress_metrics = harsher_aggregate.map(|aggregate| {
            metrics_from_aggregate(aggregate_from_gpu(aggregate, holdout_event_count))
        });
        let robust = robust_score(
            &selection_metrics,
            holdout_metrics.as_ref(),
            None,
            stress_metrics.as_ref(),
            harsher_stress_metrics.as_ref(),
            None,
        );

        RankedCompiledTuple {
            compiled: compiled_tuple.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled_tuple.tuple.clone(),
                pre_sealed_rank: 0,
                sealed_finalist: false,
                sealed_final_rank: 0,
                selection_metrics,
                holdout_metrics,
                sealed_evaluation_metrics: None,
                stress_metrics,
                sealed_stress_metrics: None,
                harsher_stress_metrics,
                sealed_harsher_stress_metrics: None,
                extreme_stress_metrics: None,
                sealed_extreme_stress_metrics: None,
                validation_verdict: Default::default(),
                robust_score: robust,
            },
            retained_summary_state: RetainedSummaryState::gpu_initial(holdout_events),
        }
    }

    fn score_gpu_lanes_legacy(
        device_id: u32,
        prepared: &PreparedOptionsExpressionRun,
        search_event_range: (usize, usize),
        holdout_event_range: (usize, usize),
    ) -> Result<
        (
            Vec<OptionsGpuTupleAggregate>,
            Option<Vec<OptionsGpuTupleAggregate>>,
            Option<Vec<OptionsGpuTupleAggregate>>,
            Option<Vec<OptionsGpuTupleAggregate>>,
            Option<Vec<f64>>,
            OptionsGpuRuntimeCacheUsage,
        ),
        String,
    > {
        let holdout_events = prepared.diagnostics.holdout_events;
        let baseline_target_ranges = if holdout_events > 0 {
            vec![search_event_range, holdout_event_range]
        } else {
            vec![search_event_range]
        };
        let baseline_tensor_run = score_options_expression_tensors_gpu_multi_range(
            device_id,
            &baseline_target_ranges,
            &prepared.baseline_gpu_inputs,
        )?;
        let mut gpu_cache_usage = baseline_tensor_run.cache_usage;
        let mut baseline_aggregates = baseline_tensor_run.aggregates_by_range;
        let search_aggregates = baseline_aggregates.swap_remove(0);
        let holdout_aggregates = if holdout_events > 0 {
            Some(
                baseline_aggregates
                    .pop()
                    .ok_or_else(|| "missing GPU holdout aggregate lane".to_string())?,
            )
        } else {
            None
        };
        let stress_aggregates = if holdout_events > 0 {
            let stress_run = score_options_expression_tensors_gpu_range(
                device_id,
                holdout_event_range.0,
                holdout_event_range.1,
                &prepared.conservative_gpu_inputs,
            )?;
            accumulate_cache_usage(&mut gpu_cache_usage, stress_run.cache_usage);
            Some(
                stress_run
                    .aggregates_by_range
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        let harsher_aggregates = if holdout_events > 0 {
            let harsher_run = score_options_expression_tensors_gpu_range(
                device_id,
                holdout_event_range.0,
                holdout_event_range.1,
                &prepared.harsher_gpu_inputs,
            )?;
            accumulate_cache_usage(&mut gpu_cache_usage, harsher_run.cache_usage);
            Some(
                harsher_run
                    .aggregates_by_range
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
            )
        } else {
            None
        };

        Ok((
            search_aggregates,
            holdout_aggregates,
            stress_aggregates,
            harsher_aggregates,
            None,
            gpu_cache_usage,
        ))
    }

    pub(super) fn rank_compiled_tuples_gpu_with_device(
        req: &OptionsExpressionRunRequest,
        prepared: &PreparedOptionsExpressionRun,
        dataset: &ImportedOptionsDataset,
        resolved_device_id: Option<u32>,
    ) -> Result<Option<GpuRankedTuplePass>, String> {
        let _nvtx_scope = crate::nvtx::scope("engine", "options_gpu_rank_total");
        if matches!(
            req.backend_preference,
            OptionsExpressionBackendPreference::CpuReference
        ) {
            return Ok(None);
        }
        let Some(device_id) = resolved_device_id else {
            if matches!(
                req.backend_preference,
                OptionsExpressionBackendPreference::GpuCompiledTensor
            ) {
                return Err(
                    "CUDA device unavailable for required GPU options-expression scorer"
                        .to_string(),
                );
            }
            return Ok(None);
        };
        let search_event_count = split_event_count(prepared, OptionsEvaluationSplit::Search);
        let holdout_events = prepared.diagnostics.holdout_events;
        let holdout_event_count = split_event_count(prepared, OptionsEvaluationSplit::Holdout);
        let search_event_range = split_event_range(prepared, OptionsEvaluationSplit::Search);
        let holdout_event_range = split_event_range(prepared, OptionsEvaluationSplit::Holdout);

        let _tensor_score_scope = crate::nvtx::scope("engine", "options_gpu_tensor_score");
        let tensor_score_start = Instant::now();
        let (
            search_aggregates,
            holdout_aggregates,
            stress_aggregates,
            harsher_aggregates,
            device_robust_scores,
            gpu_cache_usage,
        ) = if holdout_events > 0 && options_gpu_combined_scoring_enabled() {
            match score_options_expression_tensors_gpu_search_holdout_stress(
                device_id,
                search_event_range.1,
                holdout_event_range.1,
                &prepared.baseline_gpu_inputs,
                &prepared.conservative_gpu_inputs,
                &prepared.harsher_gpu_inputs,
            ) {
                Ok(combined_run) => (
                    combined_run.search_aggregates,
                    Some(combined_run.holdout_aggregates),
                    Some(combined_run.conservative_holdout_aggregates),
                    Some(combined_run.harsher_holdout_aggregates),
                    Some(combined_run.robust_scores),
                    combined_run.cache_usage,
                ),
                Err(_) => score_gpu_lanes_legacy(
                    device_id,
                    prepared,
                    search_event_range,
                    holdout_event_range,
                )?,
            }
        } else {
            score_gpu_lanes_legacy(device_id, prepared, search_event_range, holdout_event_range)?
        };
        let tensor_score_us = elapsed_us(tensor_score_start);

        let mut ranked_results: Vec<RankedCompiledTuple> = if let Some(robust_scores) =
            device_robust_scores
                .as_ref()
                .filter(|scores| scores.len() == prepared.compiled_tuples.len())
        {
            let _shortlist_scope = crate::nvtx::scope("engine", "options_gpu_host_shortlist");
            let mut shortlisted_indices: Vec<usize> =
                Vec::with_capacity(prepared.compiled_tuples.len());
            for tuple_index in 0..prepared.compiled_tuples.len() {
                let search_aggregate = search_aggregates
                    .get(tuple_index)
                    .copied()
                    .unwrap_or_default();
                if let Some(min_valid_trades) = req.min_valid_trades {
                    if (search_aggregate.valid_trades as usize) < min_valid_trades {
                        continue;
                    }
                }
                shortlisted_indices.push(tuple_index);
            }
            shortlisted_indices.sort_by(|left, right| {
                compare_gpu_shortlist_indices(*left, *right, robust_scores, &search_aggregates)
            });
            shortlisted_indices.truncate(req.top_k.max(1).min(shortlisted_indices.len()));

            let _materialize_scope =
                crate::nvtx::scope("engine", "options_gpu_host_materialize_topk");
            let mut ranked = Vec::with_capacity(shortlisted_indices.len());
            for tuple_index in shortlisted_indices {
                let compiled_tuple = &prepared.compiled_tuples[tuple_index];
                ranked.push(build_ranked_result_from_gpu_aggregates(
                    compiled_tuple,
                    search_aggregates
                        .get(tuple_index)
                        .copied()
                        .unwrap_or_default(),
                    holdout_aggregates
                        .as_ref()
                        .map(|aggregates| aggregates.get(tuple_index).copied().unwrap_or_default()),
                    stress_aggregates
                        .as_ref()
                        .map(|aggregates| aggregates.get(tuple_index).copied().unwrap_or_default()),
                    harsher_aggregates
                        .as_ref()
                        .map(|aggregates| aggregates.get(tuple_index).copied().unwrap_or_default()),
                    search_event_count,
                    holdout_event_count,
                    holdout_events,
                ));
            }
            ranked
        } else {
            let _materialize_scope =
                crate::nvtx::scope("engine", "options_gpu_host_materialize_full");
            let mut ranked = Vec::new();
            for (tuple_index, compiled_tuple) in prepared.compiled_tuples.iter().enumerate() {
                let selection_metrics = metrics_from_aggregate(aggregate_from_gpu(
                    search_aggregates
                        .get(tuple_index)
                        .copied()
                        .unwrap_or_default(),
                    search_event_count,
                ));
                if let Some(min_valid_trades) = req.min_valid_trades {
                    if selection_metrics.valid_trades < min_valid_trades {
                        continue;
                    }
                }
                let holdout_metrics = holdout_aggregates.as_ref().map(|aggregates| {
                    metrics_from_aggregate(aggregate_from_gpu(
                        aggregates.get(tuple_index).copied().unwrap_or_default(),
                        holdout_event_count,
                    ))
                });
                let stress_metrics = stress_aggregates.as_ref().map(|aggregates| {
                    metrics_from_aggregate(aggregate_from_gpu(
                        aggregates.get(tuple_index).copied().unwrap_or_default(),
                        holdout_event_count,
                    ))
                });
                let harsher_stress_metrics = harsher_aggregates.as_ref().map(|aggregates| {
                    metrics_from_aggregate(aggregate_from_gpu(
                        aggregates.get(tuple_index).copied().unwrap_or_default(),
                        holdout_event_count,
                    ))
                });
                let robust = robust_score(
                    &selection_metrics,
                    holdout_metrics.as_ref(),
                    None,
                    stress_metrics.as_ref(),
                    harsher_stress_metrics.as_ref(),
                    None,
                );
                ranked.push(RankedCompiledTuple {
                    compiled: compiled_tuple.clone(),
                    result: OptionsExpressionTupleResult {
                        tuple: compiled_tuple.tuple.clone(),
                        pre_sealed_rank: 0,
                        sealed_finalist: false,
                        sealed_final_rank: 0,
                        selection_metrics,
                        holdout_metrics,
                        sealed_evaluation_metrics: None,
                        stress_metrics,
                        sealed_stress_metrics: None,
                        harsher_stress_metrics,
                        sealed_harsher_stress_metrics: None,
                        extreme_stress_metrics: None,
                        sealed_extreme_stress_metrics: None,
                        validation_verdict: Default::default(),
                        robust_score: robust,
                    },
                    retained_summary_state: RetainedSummaryState::gpu_initial(holdout_events),
                });
            }

            sort_ranked_results(&mut ranked);
            resequence_pre_sealed_ranks(&mut ranked);
            ranked.truncate(req.top_k.max(1));
            ranked
        };
        let (exactify, sealed_stage) =
            finalize_ranked_results(req, prepared, dataset, &mut ranked_results);
        Ok(Some(GpuRankedTuplePass {
            ranked_results,
            stats: GpuRankingExecutionStats {
                timing: GpuRankingTimingBreakdown {
                    total_us: tensor_score_us
                        .saturating_add(exactify.total_us)
                        .saturating_add(sealed_stage.total_us),
                    tensor_score_us,
                    exactify,
                    sealed_stage,
                },
                cache_usage: gpu_cache_usage,
            },
        }))
    }

    pub(super) fn gpu_ranking_stats_with_device(
        req: &OptionsExpressionRunRequest,
        prepared: &PreparedOptionsExpressionRun,
        dataset: &ImportedOptionsDataset,
        resolved_device_id: Option<u32>,
    ) -> Result<Option<GpuRankingExecutionStats>, String> {
        if matches!(
            req.backend_preference,
            OptionsExpressionBackendPreference::CpuReference
        ) {
            return Ok(None);
        }
        let Some(_) = resolved_device_id else {
            if matches!(
                req.backend_preference,
                OptionsExpressionBackendPreference::GpuCompiledTensor
            ) {
                return Err(
                    "CUDA device unavailable for required GPU options-expression scorer"
                        .to_string(),
                );
            }
            return Ok(None);
        };
        if let Some(result) =
            rank_compiled_tuples_gpu_with_device(req, prepared, dataset, resolved_device_id)?
        {
            Ok(Some(result.stats))
        } else {
            Ok(None)
        }
    }
}

fn rank_compiled_tuples_cpu(
    req: &OptionsExpressionRunRequest,
    prepared: &PreparedOptionsExpressionRun,
    dataset: &ImportedOptionsDataset,
) -> Vec<RankedCompiledTuple> {
    let _nvtx_scope = crate::nvtx::scope("engine", "options_cpu_rank_total");
    let holdout_events = prepared.diagnostics.holdout_events;
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
            None,
            stress_metrics.as_ref(),
            None,
            None,
        );

        ranked_results.push(RankedCompiledTuple {
            compiled: compiled_tuple.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled_tuple.tuple.clone(),
                pre_sealed_rank: 0,
                sealed_finalist: false,
                sealed_final_rank: 0,
                selection_metrics,
                holdout_metrics,
                sealed_evaluation_metrics: None,
                stress_metrics,
                sealed_stress_metrics: None,
                harsher_stress_metrics: None,
                sealed_harsher_stress_metrics: None,
                extreme_stress_metrics: None,
                sealed_extreme_stress_metrics: None,
                validation_verdict: Default::default(),
                robust_score: robust,
            },
            retained_summary_state: RetainedSummaryState::cpu_initial(holdout_events),
        });
    }

    sort_ranked_results(&mut ranked_results);
    resequence_pre_sealed_ranks(&mut ranked_results);
    ranked_results.truncate(req.top_k.max(1));
    let _ = finalize_ranked_results(req, prepared, dataset, &mut ranked_results);
    ranked_results
}

fn rank_compiled_tuples(
    req: &OptionsExpressionRunRequest,
    prepared: &PreparedOptionsExpressionRun,
    dataset: &ImportedOptionsDataset,
) -> Result<RankedTuplePass, String> {
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let resolved_device_id = current_preferred_cuda_device_id();
        let gpu_available = resolved_device_id.is_some();
        let should_attempt_gpu = match req.backend_preference {
            OptionsExpressionBackendPreference::CpuReference => false,
            OptionsExpressionBackendPreference::GpuCompiledTensor => true,
            OptionsExpressionBackendPreference::Auto => {
                runtime_auto_backend_decision(prepared, gpu_available).gpu_eligible
            }
        };
        if should_attempt_gpu {
            match gpu_scoring::rank_compiled_tuples_gpu_with_device(
                req,
                prepared,
                dataset,
                resolved_device_id,
            ) {
                Ok(Some(gpu_ranked)) => {
                    return Ok(RankedTuplePass {
                        ranked: gpu_ranked.ranked_results,
                        backend_used: "gpu_compiled_tensor_hybrid",
                        backend_fallback_reason: None,
                    });
                }
                Ok(None) => {}
                Err(err)
                    if matches!(
                        req.backend_preference,
                        OptionsExpressionBackendPreference::GpuCompiledTensor
                    ) =>
                {
                    return Err(err);
                }
                Err(err) => {
                    return Ok(RankedTuplePass {
                        ranked: rank_compiled_tuples_cpu(req, prepared, dataset),
                        backend_used: "cpu_reference",
                        backend_fallback_reason: Some(err),
                    });
                }
            }
        }
    }

    Ok(RankedTuplePass {
        ranked: rank_compiled_tuples_cpu(req, prepared, dataset),
        backend_used: "cpu_reference",
        backend_fallback_reason: None,
    })
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
                        structure_family: axis.structure_family,
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
    fn trade_outcome_from_prices(
        net_entry_price: f64,
        net_exit_price: f64,
        premium_paid: f64,
        multiplier: f64,
    ) -> TradeOutcome {
        let pnl_per_contract = (net_exit_price - net_entry_price) * multiplier;
        TradeOutcome {
            pnl_per_contract,
            premium_paid,
            return_on_premium: pnl_per_contract / premium_paid,
        }
    }

    fn finalize_long_single_hold_at_row(
        candidate_outcomes: &mut [Option<TradeOutcome>],
        hold_index: usize,
        per_hold: usize,
        row: &OptionsPriceRow,
        net_entry_price: f64,
        premium_paid: f64,
        multiplier: f64,
        price_semantics: OptionsPriceSemantics,
        fill_model: OptionsFillModelFamily,
        stress_mode: AggregateStressMode,
    ) -> Option<()> {
        let (_, _, net_exit_price) = position_exit_prices(
            OptionsStructureFamily::LongSingle,
            row,
            None,
            price_semantics,
            fill_model,
            stress_mode,
        )?;
        let cutoff_outcome =
            trade_outcome_from_prices(net_entry_price, net_exit_price, premium_paid, multiplier);
        let hold_start = hold_index * per_hold;
        for slot in candidate_outcomes[hold_start..(hold_start + per_hold)].iter_mut() {
            if slot.is_none() {
                *slot = Some(cutoff_outcome.clone());
            }
        }
        Some(())
    }

    fn compile_long_single_candidate_outcomes(
        candidate_outcomes: &mut [Option<TradeOutcome>],
        event: &OptionsSignalEvent,
        contract: &OptionsContractSeries,
        entry_index: usize,
        axis: &ExitAxis,
        price_semantics: OptionsPriceSemantics,
        stress_mode: AggregateStressMode,
    ) -> Option<()> {
        let contract_rows = &contract.rows;
        let entry_row = contract_rows.get(entry_index)?;
        let (_, _, net_entry_price) = position_entry_prices(
            OptionsStructureFamily::LongSingle,
            entry_row,
            None,
            price_semantics,
            axis.fill_model,
            stress_mode,
        )?;
        if !net_entry_price.is_finite() || net_entry_price <= 0.0 {
            return None;
        }
        let premium_paid = net_entry_price * contract.multiplier;
        let target_prices: Vec<Option<f64>> = axis
            .target_values
            .iter()
            .copied()
            .map(|target| target.map(|pct| net_entry_price * (1.0 + pct.max(0.0))))
            .collect();
        let stop_prices: Vec<Option<f64>> = axis
            .stop_values
            .iter()
            .copied()
            .map(|stop| stop.map(|pct| net_entry_price * (1.0 - pct.max(0.0))))
            .collect();
        let per_hold = target_prices.len() * stop_prices.len();
        debug_assert_eq!(candidate_outcomes.len(), axis.hold_minutes.len() * per_hold);
        let forced_expiry_cutoff = contract.expiration_timestamp_ms.saturating_sub(MINUTE_MS);
        let mut hold_cutoffs = Vec::with_capacity(axis.hold_minutes.len());
        let mut max_cutoff_index = entry_index;
        for (hold_index, hold_minutes) in axis.hold_minutes.iter().copied().enumerate() {
            let requested_cutoff = match axis.exit_template {
                OptionsExitTemplateFamily::FixedHold
                | OptionsExitTemplateFamily::FixedHoldOrTargetStop => {
                    entry_row.timestamp_ms + i64::from(hold_minutes) * MINUTE_MS
                }
                OptionsExitTemplateFamily::EndOfDayOrTargetStop => {
                    resolve_eod_cutoff(contract_rows, entry_index)
                }
                OptionsExitTemplateFamily::BaseExitOrTargetStop => {
                    event.base_exit_timestamp.unwrap_or_else(|| {
                        entry_row.timestamp_ms + i64::from(hold_minutes) * MINUTE_MS
                    })
                }
            };
            let cutoff_timestamp_ms = requested_cutoff.min(forced_expiry_cutoff);
            if let Some(cutoff_index) =
                find_cutoff_index(contract_rows, entry_index, cutoff_timestamp_ms)
            {
                max_cutoff_index = max_cutoff_index.max(cutoff_index);
                hold_cutoffs.push((cutoff_index, hold_index));
            }
        }
        hold_cutoffs
            .sort_unstable_by_key(|(cutoff_index, hold_index)| (*cutoff_index, *hold_index));

        let stop_count = stop_prices.len();
        let mut next_cutoff = 0usize;
        while next_cutoff < hold_cutoffs.len() && hold_cutoffs[next_cutoff].0 == entry_index {
            finalize_long_single_hold_at_row(
                candidate_outcomes,
                hold_cutoffs[next_cutoff].1,
                per_hold,
                entry_row,
                net_entry_price,
                premium_paid,
                contract.multiplier,
                price_semantics,
                axis.fill_model,
                stress_mode,
            )?;
            next_cutoff += 1;
        }
        if next_cutoff == hold_cutoffs.len() {
            return Some(());
        }

        for idx in (entry_index + 1)..=max_cutoff_index {
            let row = contract_rows.get(idx)?;
            let (option_high, option_low) = position_path_range_for_rows(
                OptionsStructureFamily::LongSingle,
                row,
                None,
                price_semantics,
                axis.fill_model,
                stress_mode,
            )?;
            let target_hits: Vec<bool> = target_prices
                .iter()
                .map(|target| target.map(|price| option_high >= price).unwrap_or(false))
                .collect();
            let stop_hits: Vec<bool> = stop_prices
                .iter()
                .map(|stop| stop.map(|price| option_low <= price).unwrap_or(false))
                .collect();
            if !target_hits.iter().any(|hit| *hit) && !stop_hits.iter().any(|hit| *hit) {
                while next_cutoff < hold_cutoffs.len() && hold_cutoffs[next_cutoff].0 == idx {
                    finalize_long_single_hold_at_row(
                        candidate_outcomes,
                        hold_cutoffs[next_cutoff].1,
                        per_hold,
                        row,
                        net_entry_price,
                        premium_paid,
                        contract.multiplier,
                        price_semantics,
                        axis.fill_model,
                        stress_mode,
                    )?;
                    next_cutoff += 1;
                }
                if next_cutoff == hold_cutoffs.len() {
                    break;
                }
                continue;
            }

            for &(_, hold_index) in hold_cutoffs.iter().skip(next_cutoff) {
                let hold_start = hold_index * per_hold;
                for (target_index, target_hit) in target_hits.iter().copied().enumerate() {
                    for (stop_index, stop_hit) in stop_hits.iter().copied().enumerate() {
                        if !stop_hit && !target_hit {
                            continue;
                        }
                        let local_index = target_index * stop_count + stop_index;
                        let slot = &mut candidate_outcomes[hold_start + local_index];
                        if slot.is_some() {
                            continue;
                        }
                        let net_exit_price = if stop_hit {
                            stop_prices[stop_index].expect("stop hit requires stop threshold")
                        } else {
                            target_prices[target_index]
                                .expect("target hit requires target threshold")
                        };
                        *slot = Some(trade_outcome_from_prices(
                            net_entry_price,
                            net_exit_price,
                            premium_paid,
                            contract.multiplier,
                        ));
                    }
                }
            }

            while next_cutoff < hold_cutoffs.len() && hold_cutoffs[next_cutoff].0 == idx {
                finalize_long_single_hold_at_row(
                    candidate_outcomes,
                    hold_cutoffs[next_cutoff].1,
                    per_hold,
                    row,
                    net_entry_price,
                    premium_paid,
                    contract.multiplier,
                    price_semantics,
                    axis.fill_model,
                    stress_mode,
                )?;
                next_cutoff += 1;
            }
            if next_cutoff == hold_cutoffs.len() {
                break;
            }
        }
        Some(())
    }

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
                    let candidate_start = axis.summary_index(candidate_slot_index, 0, 0, 0);
                    let candidate_len = axis.summary_count_per_candidate();
                    let candidate_outcomes =
                        &mut outcomes[candidate_start..(candidate_start + candidate_len)];
                    if matches!(
                        candidate_cache.axis.structure_family,
                        OptionsStructureFamily::LongSingle
                    ) {
                        let contract = index.contract(candidate.primary_leg.contract_index);
                        let _ = compile_long_single_candidate_outcomes(
                            candidate_outcomes,
                            event,
                            contract,
                            candidate.primary_leg.entry_index,
                            &axis,
                            dataset.manifest.price_semantics,
                            stress_mode,
                        );
                        continue;
                    }
                    for (hold_index, hold_minutes) in axis.hold_minutes.iter().copied().enumerate()
                    {
                        let hold_start =
                            hold_index * axis.target_values.len() * axis.stop_values.len();
                        let hold_outcomes = &mut candidate_outcomes[hold_start
                            ..(hold_start + axis.target_values.len() * axis.stop_values.len())];
                        for (target_index, target_return) in
                            axis.target_values.iter().copied().enumerate()
                        {
                            for (stop_index, stop_return) in
                                axis.stop_values.iter().copied().enumerate()
                            {
                                let local_index =
                                    target_index * axis.stop_values.len() + stop_index;
                                let eval_tuple = OptionsTupleSpec {
                                    structure_family: candidate_cache.axis.structure_family,
                                    delay_minutes: 0,
                                    dte_days: 0,
                                    moneyness_bucket: OptionsMoneynessBucket::Atm,
                                    hold_minutes,
                                    target_return,
                                    stop_return,
                                    exit_template: axis.exit_template,
                                    fill_model: axis.fill_model,
                                };
                                hold_outcomes[local_index] = evaluate_trade_summary(
                                    event,
                                    &eval_tuple,
                                    candidate,
                                    &index,
                                    dataset.manifest.price_semantics,
                                    stress_mode,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    ExitSummaryCache { axis, outcomes }
}

fn get_or_build_structural_candidate_cache(
    signal_manifest: &OptionsSignalEventManifestV1,
    events: &[OptionsSignalEvent],
    dataset: &ImportedOptionsDataset,
    axis: CandidateAxis,
    fill_model: OptionsFillModelFamily,
    filters: &crate::options_types::OptionsStaticFilterBundle,
    stress_mode: AggregateStressMode,
    cache_stats: &mut OptionsExpressionFeasibilityCacheStats,
) -> CacheLookup<StructuralCandidateCache> {
    let key = structural_candidate_cache_key(
        signal_manifest,
        dataset,
        &axis,
        fill_model,
        filters,
        stress_mode,
    );
    {
        let cache = structural_candidate_cache_store()
            .lock()
            .expect("structural candidate cache poisoned");
        if let Some(existing) = cache.get(&key).cloned() {
            cache_stats.structural_cache_hits += 1;
            cache_stats.structural_cache_entries = cache.len();
            return CacheLookup {
                value: existing,
                hit: true,
            };
        }
    }
    let built =
        compile_structural_candidate_cache(events, dataset, axis, fill_model, filters, stress_mode);
    let mut cache = structural_candidate_cache_store()
        .lock()
        .expect("structural candidate cache poisoned");
    evict_if_needed(&mut cache);
    cache.insert(key, built.clone());
    cache_stats.structural_cache_misses += 1;
    cache_stats.structural_cache_entries = cache.len();
    CacheLookup {
        value: built,
        hit: false,
    }
}

fn get_or_build_exit_summary_cache(
    structural_key: &str,
    events: &[OptionsSignalEvent],
    dataset: &ImportedOptionsDataset,
    candidate_cache: &StructuralCandidateCache,
    axis: ExitAxis,
    stress_mode: AggregateStressMode,
    cache_stats: &mut OptionsExpressionFeasibilityCacheStats,
) -> CacheLookup<ExitSummaryCache> {
    let key = exit_summary_cache_key(structural_key, &axis);
    {
        let cache = exit_summary_cache_store()
            .lock()
            .expect("exit summary cache poisoned");
        if let Some(existing) = cache.get(&key).cloned() {
            cache_stats.summary_cache_hits += 1;
            cache_stats.summary_cache_entries = cache.len();
            return CacheLookup {
                value: existing,
                hit: true,
            };
        }
    }
    let built = compile_exit_summary_cache(events, dataset, candidate_cache, axis, stress_mode);
    let mut cache = exit_summary_cache_store()
        .lock()
        .expect("exit summary cache poisoned");
    evict_if_needed(&mut cache);
    cache.insert(key, built.clone());
    cache_stats.summary_cache_misses += 1;
    cache_stats.summary_cache_entries = cache.len();
    CacheLookup {
        value: built,
        hit: false,
    }
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

fn evaluate_median_return_on_premium_for_split(
    tuple: &CompiledTupleSpec,
    events: &[OptionsSignalEvent],
    split: OptionsEvaluationSplit,
    candidate_cache: &StructuralCandidateCache,
    exit_summary_cache: &ExitSummaryCache,
) -> f64 {
    let mut returns: Vec<f64> = Vec::new();
    for (event_index, event) in events.iter().enumerate() {
        if event.fold != split {
            continue;
        }
        let candidate_slot_index = candidate_cache.candidate_slot_index(
            event_index,
            tuple.delay_index,
            tuple.dte_index,
            tuple.moneyness_index,
        );
        if let Some(outcome) = exit_summary_cache.outcome(
            candidate_slot_index,
            tuple.hold_index,
            tuple.target_index,
            tuple.stop_index,
        ) {
            returns.push(outcome.return_on_premium);
        }
    }
    median(returns)
}

fn evaluate_exact_stress_for_split(
    tuple: &CompiledTupleSpec,
    events: &[OptionsSignalEvent],
    split: OptionsEvaluationSplit,
    index: &IndexedOptionsDataset<'_>,
    candidate_cache: &StructuralCandidateCache,
    price_semantics: OptionsPriceSemantics,
    stress_mode: AggregateStressMode,
) -> OptionsExpressionMetrics {
    let mut split_event_count = 0usize;
    let mut outcomes: Vec<TradeOutcome> = Vec::new();
    for (event_index, event) in events.iter().enumerate() {
        if event.fold != split {
            continue;
        }
        split_event_count += 1;
        let Some(candidate) = candidate_cache.candidate(
            event_index,
            tuple.delay_index,
            tuple.dte_index,
            tuple.moneyness_index,
        ) else {
            continue;
        };
        if let Some(outcome) = evaluate_trade_summary(
            event,
            &tuple.tuple,
            candidate,
            index,
            price_semantics,
            stress_mode,
        ) {
            outcomes.push(outcome);
        }
    }
    compute_metrics(split_event_count, &outcomes)
}

fn robust_score(
    search: &OptionsExpressionMetrics,
    holdout: Option<&OptionsExpressionMetrics>,
    _sealed: Option<&OptionsExpressionMetrics>,
    stress: Option<&OptionsExpressionMetrics>,
    harsher_stress: Option<&OptionsExpressionMetrics>,
    extreme_stress: Option<&OptionsExpressionMetrics>,
) -> f64 {
    fn lane_utility(metrics: &OptionsExpressionMetrics) -> f64 {
        (1.0 - metrics.invalid_rate).clamp(0.0, 1.0) * metrics.expectancy_per_premium
    }

    let search_score = lane_utility(search);
    let holdout_score = holdout.map(lane_utility).unwrap_or(search_score);
    let stress_score = stress.map(lane_utility).unwrap_or(search_score);
    let harsher_stress_score = harsher_stress.map(lane_utility).unwrap_or(stress_score);
    let extreme_stress_score = extreme_stress
        .map(lane_utility)
        .unwrap_or(harsher_stress_score);
    // Sealed metrics are recorded for final reporting, not for tuple ranking.
    0.45 * search_score
        + 0.25 * holdout_score
        + 0.15 * stress_score
        + 0.10 * harsher_stress_score
        + 0.05 * extreme_stress_score
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
            candidate_invalid_reason: candidate_cache.invalid_reason(
                event_index,
                compiled_tuple.delay_index,
                compiled_tuple.dte_index,
                compiled_tuple.moneyness_index,
            ),
            contract_id: None,
            companion_contract_id: None,
            expiration_date: None,
            strike: None,
            companion_strike: None,
            multiplier: None,
            entry_timestamp: None,
            entry_option_price: None,
            companion_entry_option_price: None,
            entry_underlier_price: None,
            exit_timestamp: None,
            exit_option_price: None,
            companion_exit_option_price: None,
            premium_paid: None,
            pnl_per_contract: None,
            return_on_premium: None,
            entry_lag_minutes: None,
            minutes_to_expiry_at_entry: None,
            realized_hold_minutes: None,
            path_option_high: None,
            path_option_low: None,
            path_underlier_high: None,
            path_underlier_low: None,
            exit_reason: None,
        };
    };

    let contract = index.contract(candidate.primary_leg.contract_index);
    let companion_contract = candidate
        .companion_leg
        .map(|leg| index.contract(leg.contract_index));
    let entry_row = contract.rows.get(candidate.primary_leg.entry_index);
    let fallback_entry_price = entry_row.and_then(|row| {
        option_entry_price_for_row(
            row,
            index.dataset.manifest.price_semantics,
            compiled_tuple.tuple.fill_model,
        )
    });
    let fallback_companion_entry_price = candidate
        .companion_leg
        .and_then(|leg| companion_contract?.rows.get(leg.entry_index))
        .and_then(|row| {
            option_exit_price_for_row(
                row,
                index.dataset.manifest.price_semantics,
                compiled_tuple.tuple.fill_model,
                AggregateStressMode::Baseline,
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
    let entry_timestamp = detail
        .as_ref()
        .map(|value| value.entry_timestamp_ms)
        .or_else(|| entry_row.map(|row| row.timestamp_ms));
    let entry_underlier_price = detail
        .as_ref()
        .map(|value| value.entry_underlier_price)
        .or_else(|| entry_row.map(|row| row.underlying_price));
    let entry_lag_minutes =
        entry_timestamp.map(|ts| ts.saturating_sub(event.earliest_tradable_timestamp) / MINUTE_MS);
    let minutes_to_expiry_at_entry =
        entry_timestamp.map(|ts| contract.expiration_timestamp_ms.saturating_sub(ts) / MINUTE_MS);
    let realized_hold_minutes = detail.as_ref().map(|value| {
        value
            .exit_timestamp_ms
            .saturating_sub(value.entry_timestamp_ms)
            / MINUTE_MS
    });

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
        candidate_invalid_reason: None,
        contract_id: Some(contract.contract_id.clone()),
        companion_contract_id: companion_contract.map(|contract| contract.contract_id.clone()),
        expiration_date: Some(contract.expiration_date.clone()),
        strike: Some(contract.strike),
        companion_strike: companion_contract.map(|contract| contract.strike),
        multiplier: Some(contract.multiplier),
        entry_timestamp,
        entry_option_price: detail
            .as_ref()
            .map(|value| value.entry_price)
            .or(fallback_entry_price),
        companion_entry_option_price: detail
            .as_ref()
            .and_then(|value| value.companion_entry_price)
            .or(fallback_companion_entry_price),
        entry_underlier_price,
        exit_timestamp: detail.as_ref().map(|value| value.exit_timestamp_ms),
        exit_option_price: detail.as_ref().map(|value| value.exit_price),
        companion_exit_option_price: detail.as_ref().and_then(|value| value.companion_exit_price),
        premium_paid: detail.as_ref().map(|value| value.premium_paid),
        pnl_per_contract: detail.as_ref().map(|value| value.pnl_per_contract),
        return_on_premium: detail.as_ref().map(|value| value.return_on_premium),
        entry_lag_minutes,
        minutes_to_expiry_at_entry,
        realized_hold_minutes,
        path_option_high: detail.as_ref().map(|value| value.path_option_high),
        path_option_low: detail.as_ref().map(|value| value.path_option_low),
        path_underlier_high: detail.as_ref().map(|value| value.path_underlier_high),
        path_underlier_low: detail.as_ref().map(|value| value.path_underlier_low),
        exit_reason: detail.as_ref().map(|value| value.exit_reason),
    }
}

fn build_event_path_points(
    structure_family: OptionsStructureFamily,
    primary_contract: &OptionsContractSeries,
    companion_contract: Option<&OptionsContractSeries>,
    start_index: usize,
    companion_start_index: Option<usize>,
    end_index: usize,
) -> Vec<OptionsExpressionEventPathPoint> {
    let Some(end_index) = (start_index <= end_index).then_some(end_index) else {
        return Vec::new();
    };
    if let Some(companion_contract) = companion_contract {
        let mut points = Vec::new();
        let mut primary_index = start_index;
        let mut companion_index = companion_start_index.unwrap_or(0);
        while primary_index <= end_index
            && primary_index < primary_contract.rows.len()
            && companion_index < companion_contract.rows.len()
        {
            let primary_row = &primary_contract.rows[primary_index];
            let companion_row = &companion_contract.rows[companion_index];
            match primary_row.timestamp_ms.cmp(&companion_row.timestamp_ms) {
                Ordering::Less => primary_index += 1,
                Ordering::Greater => companion_index += 1,
                Ordering::Equal => {
                    let (option_open, option_high, option_low, option_close) =
                        match structure_family {
                            OptionsStructureFamily::LongSingle => (
                                primary_row.open,
                                primary_row.high,
                                primary_row.low,
                                primary_row.close,
                            ),
                            OptionsStructureFamily::DebitVertical => (
                                (primary_row.open - companion_row.open).max(0.0),
                                (primary_row.high - companion_row.low).max(0.0),
                                (primary_row.low - companion_row.high).max(0.0),
                                (primary_row.close - companion_row.close).max(0.0),
                            ),
                            OptionsStructureFamily::LongPair => (
                                (primary_row.open + companion_row.open).max(0.0),
                                (primary_row.high + companion_row.high).max(0.0),
                                (primary_row.low + companion_row.low).max(0.0),
                                (primary_row.close + companion_row.close).max(0.0),
                            ),
                        };
                    points.push(OptionsExpressionEventPathPoint {
                        timestamp_ms: primary_row.timestamp_ms,
                        option_open,
                        option_high,
                        option_low,
                        option_close,
                        underlying_price: primary_row.underlying_price,
                    });
                    primary_index += 1;
                    companion_index += 1;
                }
            }
        }
        points
    } else {
        primary_contract
            .rows
            .iter()
            .enumerate()
            .skip(start_index)
            .take(end_index.saturating_sub(start_index) + 1)
            .map(|(_, row)| OptionsExpressionEventPathPoint {
                timestamp_ms: row.timestamp_ms,
                option_open: row.open,
                option_high: row.high,
                option_low: row.low,
                option_close: row.close,
                underlying_price: row.underlying_price,
            })
            .collect()
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

fn build_event_coverage_summary(
    prepared: &PreparedOptionsExpressionRun,
) -> OptionsExpressionFeasibilityEventCoverage {
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
    let (
        total_events_with_harsher_outcomes,
        search_events_with_harsher_outcomes,
        holdout_events_with_harsher_outcomes,
        sealed_test_events_with_harsher_outcomes,
    ) = split_coverage_counts(&prepared.events, |event_index| {
        event_has_any_outcome(
            &prepared.harsher_candidate_cache,
            &prepared.harsher_exit_summary_cache,
            event_index,
        )
    });
    OptionsExpressionFeasibilityEventCoverage {
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
        total_events_with_harsher_outcomes,
        search_events_with_harsher_outcomes,
        holdout_events_with_harsher_outcomes,
        sealed_test_events_with_harsher_outcomes,
    }
}

fn summarize_event_split_counts(
    events: &[OptionsSignalEvent],
    predicate: impl Fn(&OptionsSignalEvent) -> bool,
) -> (usize, usize, usize, usize) {
    let mut total = 0usize;
    let mut search = 0usize;
    let mut holdout = 0usize;
    let mut sealed = 0usize;
    for event in events {
        if !predicate(event) {
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

fn parse_contract_reference_timestamp_ms(value: Option<&str>) -> Option<i64> {
    value
        .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .map(|ts| ts.timestamp_millis())
}

fn build_overlap_window_summary(
    events: &[OptionsSignalEvent],
    diagnostics: &OptionsExpressionDiagnostics,
    dataset: &ImportedOptionsDataset,
) -> OptionsExpressionOverlapWindowSummary {
    let coverage_start = dataset.manifest.coverage_start;
    let coverage_end = dataset.manifest.coverage_end;
    let (
        total_events_in_dataset_window,
        search_events_in_dataset_window,
        holdout_events_in_dataset_window,
        sealed_test_events_in_dataset_window,
    ) = summarize_event_split_counts(events, |event| {
        let basis_ts = event.earliest_tradable_timestamp;
        basis_ts >= coverage_start && basis_ts <= coverage_end
    });
    let (
        total_events_before_dataset_window,
        search_events_before_dataset_window,
        holdout_events_before_dataset_window,
        sealed_test_events_before_dataset_window,
    ) = summarize_event_split_counts(events, |event| {
        event.earliest_tradable_timestamp < coverage_start
    });
    let (
        total_events_after_dataset_window,
        search_events_after_dataset_window,
        holdout_events_after_dataset_window,
        sealed_test_events_after_dataset_window,
    ) = summarize_event_split_counts(events, |event| {
        event.earliest_tradable_timestamp > coverage_end
    });
    let contract_reference_as_of_utc = dataset.manifest.contract_reference_as_of_utc.clone();
    let contract_reference_as_of_ms =
        parse_contract_reference_timestamp_ms(contract_reference_as_of_utc.as_deref());
    let (
        total_events_before_contract_reference_as_of,
        search_events_before_contract_reference_as_of,
        holdout_events_before_contract_reference_as_of,
        sealed_test_events_before_contract_reference_as_of,
    ) = if let Some(as_of_ms) = contract_reference_as_of_ms {
        summarize_event_split_counts(events, |event| event.earliest_tradable_timestamp < as_of_ms)
    } else {
        (0, 0, 0, 0)
    };
    OptionsExpressionOverlapWindowSummary {
        contract_reference_as_of_utc,
        total_events_in_dataset_window,
        search_events_in_dataset_window,
        holdout_events_in_dataset_window,
        sealed_test_events_in_dataset_window,
        total_events_before_dataset_window,
        total_events_after_dataset_window,
        search_events_before_dataset_window,
        search_events_after_dataset_window,
        holdout_events_before_dataset_window,
        holdout_events_after_dataset_window,
        sealed_test_events_before_dataset_window,
        sealed_test_events_after_dataset_window,
        total_events_before_contract_reference_as_of,
        search_events_before_contract_reference_as_of,
        holdout_events_before_contract_reference_as_of,
        sealed_test_events_before_contract_reference_as_of,
        total_overlap_rate: ratio_or_zero(total_events_in_dataset_window, diagnostics.total_events),
        search_overlap_rate: ratio_or_zero(
            search_events_in_dataset_window,
            diagnostics.search_events,
        ),
        holdout_overlap_rate: ratio_or_zero(
            holdout_events_in_dataset_window,
            diagnostics.holdout_events,
        ),
        sealed_test_overlap_rate: ratio_or_zero(
            sealed_test_events_in_dataset_window,
            diagnostics.sealed_test_events,
        ),
    }
}

fn build_split_invalid_reason_summary(
    events: &[OptionsSignalEvent],
    candidate_cache: &StructuralCandidateCache,
) -> OptionsExpressionSplitInvalidReasonSummary {
    let per_event = candidate_cache.axis.slot_count_per_event();
    if per_event == 0 || events.is_empty() {
        return OptionsExpressionSplitInvalidReasonSummary::default();
    }
    let mut search_counts: BTreeMap<OptionsCandidateInvalidReason, usize> = BTreeMap::new();
    let mut holdout_counts: BTreeMap<OptionsCandidateInvalidReason, usize> = BTreeMap::new();
    let mut sealed_counts: BTreeMap<OptionsCandidateInvalidReason, usize> = BTreeMap::new();
    for (slot_index, reason) in candidate_cache.invalid_reasons.iter().enumerate() {
        let Some(reason) = reason else {
            continue;
        };
        let event_index = slot_index / per_event;
        let Some(event) = events.get(event_index) else {
            continue;
        };
        let counts = match event.fold {
            OptionsEvaluationSplit::Search => &mut search_counts,
            OptionsEvaluationSplit::Holdout => &mut holdout_counts,
            OptionsEvaluationSplit::SealedTest => &mut sealed_counts,
        };
        *counts.entry(*reason).or_default() += 1;
    }
    let map_counts = |counts: BTreeMap<OptionsCandidateInvalidReason, usize>| {
        counts
            .into_iter()
            .map(|(reason, count)| OptionsCandidateInvalidReasonCount { reason, count })
            .collect()
    };
    OptionsExpressionSplitInvalidReasonSummary {
        search_invalid_reasons: map_counts(search_counts),
        holdout_invalid_reasons: map_counts(holdout_counts),
        sealed_test_invalid_reasons: map_counts(sealed_counts),
    }
}

fn dominant_invalid_reason(
    invalid_reasons: &[OptionsCandidateInvalidReasonCount],
) -> Option<(OptionsCandidateInvalidReason, f64)> {
    let total: usize = invalid_reasons.iter().map(|entry| entry.count).sum();
    let best = invalid_reasons.iter().max_by_key(|entry| entry.count)?;
    let share = if total == 0 {
        0.0
    } else {
        best.count as f64 / total as f64
    };
    Some((best.reason, share))
}

fn push_adequacy_reason(reasons: &mut Vec<String>, reason: impl Into<String>) {
    let next = reason.into();
    if !reasons.iter().any(|existing| existing == &next) {
        reasons.push(next);
    }
}

fn build_coverage_adequacy_summary(
    diagnostics: &OptionsExpressionDiagnostics,
    overlap_window: &OptionsExpressionOverlapWindowSummary,
    event_coverage: &OptionsExpressionFeasibilityEventCoverage,
    split_invalid_reasons: &OptionsExpressionSplitInvalidReasonSummary,
    invalid_reasons: &[OptionsCandidateInvalidReasonCount],
) -> OptionsExpressionCoverageAdequacySummary {
    let search_candidate_coverage_rate = ratio_or_zero(
        event_coverage.search_events_with_candidates,
        overlap_window.search_events_in_dataset_window,
    );
    let holdout_candidate_coverage_rate = ratio_or_zero(
        event_coverage.holdout_events_with_candidates,
        overlap_window.holdout_events_in_dataset_window,
    );
    let sealed_test_candidate_coverage_rate = ratio_or_zero(
        event_coverage.sealed_test_events_with_candidates,
        overlap_window.sealed_test_events_in_dataset_window,
    );
    let search_baseline_outcome_coverage_rate = ratio_or_zero(
        event_coverage.search_events_with_baseline_outcomes,
        overlap_window.search_events_in_dataset_window,
    );
    let holdout_baseline_outcome_coverage_rate = ratio_or_zero(
        event_coverage.holdout_events_with_baseline_outcomes,
        overlap_window.holdout_events_in_dataset_window,
    );
    let sealed_test_baseline_outcome_coverage_rate = ratio_or_zero(
        event_coverage.sealed_test_events_with_baseline_outcomes,
        overlap_window.sealed_test_events_in_dataset_window,
    );
    let dominant_search_invalid =
        dominant_invalid_reason(&split_invalid_reasons.search_invalid_reasons);
    let dominant_overall_invalid = dominant_invalid_reason(invalid_reasons);
    let (dominant_invalid_reason, dominant_invalid_reason_share) = dominant_search_invalid
        .or(dominant_overall_invalid)
        .unwrap_or((OptionsCandidateInvalidReason::NoPriceRowAfterEntry, 0.0));
    let dominant_invalid_reason = if dominant_invalid_reason_share > 0.0 {
        Some(dominant_invalid_reason)
    } else {
        None
    };
    let mut reasons = Vec::new();
    let mut verdict = OptionsExpressionCoverageAdequacyVerdict::Healthy;
    let mut non_promotable = false;

    if diagnostics.search_events > 0 && overlap_window.search_events_in_dataset_window == 0 {
        verdict = OptionsExpressionCoverageAdequacyVerdict::SearchOverlapMissing;
        non_promotable = true;
        push_adequacy_reason(
            &mut reasons,
            "No search events fall inside the imported options dataset window.",
        );
    } else if diagnostics.search_events > 0
        && (overlap_window.search_overlap_rate < OPTIONS_MIN_HEALTHY_SEARCH_OVERLAP_RATE
            || overlap_window.total_overlap_rate < OPTIONS_MIN_HEALTHY_TOTAL_OVERLAP_RATE)
    {
        verdict = OptionsExpressionCoverageAdequacyVerdict::WeakDatasetOverlap;
        non_promotable = true;
        push_adequacy_reason(
            &mut reasons,
            format!(
                "Only {:.1}% of search events and {:.1}% of total events overlap the current dataset window.",
                overlap_window.search_overlap_rate * 100.0,
                overlap_window.total_overlap_rate * 100.0
            ),
        );
    } else if overlap_window.search_events_in_dataset_window > 0
        && (search_candidate_coverage_rate < OPTIONS_MIN_HEALTHY_SEARCH_CANDIDATE_COVERAGE_RATE
            || search_baseline_outcome_coverage_rate
                < OPTIONS_MIN_HEALTHY_SEARCH_BASELINE_OUTCOME_COVERAGE_RATE)
    {
        verdict = OptionsExpressionCoverageAdequacyVerdict::SearchCoverageWeak;
        non_promotable = true;
        push_adequacy_reason(
            &mut reasons,
            format!(
                "Only {:.1}% of in-window search events reach candidates and {:.1}% reach baseline outcomes.",
                search_candidate_coverage_rate * 100.0,
                search_baseline_outcome_coverage_rate * 100.0
            ),
        );
    } else if diagnostics.holdout_events > 0
        && overlap_window.holdout_events_in_dataset_window > 0
        && (holdout_candidate_coverage_rate < OPTIONS_MIN_HEALTHY_HOLDOUT_COVERAGE_RATE
            || holdout_baseline_outcome_coverage_rate < OPTIONS_MIN_HEALTHY_HOLDOUT_COVERAGE_RATE)
    {
        verdict = OptionsExpressionCoverageAdequacyVerdict::HoldoutCoverageWeak;
        push_adequacy_reason(
            &mut reasons,
            format!(
                "Holdout coverage is thin: {:.1}% candidate-ready and {:.1}% baseline-outcome coverage for in-window holdout events.",
                holdout_candidate_coverage_rate * 100.0,
                holdout_baseline_outcome_coverage_rate * 100.0
            ),
        );
    } else if diagnostics.sealed_test_events > 0
        && overlap_window.sealed_test_events_in_dataset_window > 0
        && (sealed_test_candidate_coverage_rate < OPTIONS_MIN_HEALTHY_SEALED_COVERAGE_RATE
            || sealed_test_baseline_outcome_coverage_rate
                < OPTIONS_MIN_HEALTHY_SEALED_COVERAGE_RATE)
    {
        verdict = OptionsExpressionCoverageAdequacyVerdict::SealedCoverageWeak;
        push_adequacy_reason(
            &mut reasons,
            format!(
                "Sealed coverage is thin: {:.1}% candidate-ready and {:.1}% baseline-outcome coverage for in-window sealed events.",
                sealed_test_candidate_coverage_rate * 100.0,
                sealed_test_baseline_outcome_coverage_rate * 100.0
            ),
        );
    } else if dominant_invalid_reason_share >= OPTIONS_DOMINANT_INVALID_REASON_SHARE_THRESHOLD {
        match dominant_invalid_reason {
            Some(OptionsCandidateInvalidReason::ContractNotEligibleAsOfEvent) => {
                verdict = OptionsExpressionCoverageAdequacyVerdict::ContractEligibilityFragile;
                push_adequacy_reason(
                    &mut reasons,
                    format!(
                        "Contract eligibility timing dominates structural invalidity ({:.1}% of invalid slots).",
                        dominant_invalid_reason_share * 100.0
                    ),
                );
                if overlap_window.search_events_before_contract_reference_as_of > 0 {
                    push_adequacy_reason(
                        &mut reasons,
                        format!(
                            "{} search event{} occur before the dataset contract-reference as-of timestamp.",
                            overlap_window.search_events_before_contract_reference_as_of,
                            if overlap_window.search_events_before_contract_reference_as_of == 1 {
                                ""
                            } else {
                                "s"
                            }
                        ),
                    );
                }
            }
            Some(OptionsCandidateInvalidReason::EntryPastCutoff) => {
                verdict = OptionsExpressionCoverageAdequacyVerdict::LateEntryFragile;
                push_adequacy_reason(
                    &mut reasons,
                    format!(
                        "Late-entry cutoff misses dominate structural invalidity ({:.1}% of invalid slots).",
                        dominant_invalid_reason_share * 100.0
                    ),
                );
            }
            Some(OptionsCandidateInvalidReason::NoStrikeInTolerance) => {
                verdict = OptionsExpressionCoverageAdequacyVerdict::StrikeWindowFragile;
                push_adequacy_reason(
                    &mut reasons,
                    format!(
                        "Strike-window misses dominate structural invalidity ({:.1}% of invalid slots).",
                        dominant_invalid_reason_share * 100.0
                    ),
                );
            }
            _ => {}
        }
    }

    if verdict == OptionsExpressionCoverageAdequacyVerdict::Healthy {
        if overlap_window.search_events_before_contract_reference_as_of > 0
            || dominant_invalid_reason_share >= OPTIONS_DOMINANT_INVALID_REASON_SHARE_THRESHOLD
        {
            verdict = OptionsExpressionCoverageAdequacyVerdict::MixedFragile;
            if overlap_window.search_events_before_contract_reference_as_of > 0 {
                push_adequacy_reason(
                    &mut reasons,
                    format!(
                        "{} search event{} occur before the dataset contract-reference as-of timestamp.",
                        overlap_window.search_events_before_contract_reference_as_of,
                        if overlap_window.search_events_before_contract_reference_as_of == 1 {
                            ""
                        } else {
                            "s"
                        }
                    ),
                );
            }
            if let Some(reason) = dominant_invalid_reason {
                push_adequacy_reason(
                    &mut reasons,
                    format!(
                        "Dominant structural invalid reason: {:?} ({:.1}% of invalid slots).",
                        reason,
                        dominant_invalid_reason_share * 100.0
                    ),
                );
            }
        } else {
            push_adequacy_reason(
                &mut reasons,
                "Search overlap and split-level candidate/outcome coverage are currently adequate for this slice.",
            );
        }
    }

    OptionsExpressionCoverageAdequacySummary {
        verdict,
        non_promotable,
        reasons,
        search_candidate_coverage_rate,
        holdout_candidate_coverage_rate,
        sealed_test_candidate_coverage_rate,
        search_baseline_outcome_coverage_rate,
        holdout_baseline_outcome_coverage_rate,
        sealed_test_baseline_outcome_coverage_rate,
        dominant_invalid_reason,
        dominant_invalid_reason_share,
    }
}

pub fn run_options_expression_blocking(
    req: OptionsExpressionRunRequest,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionRunResult, String> {
    let _nvtx_scope = crate::nvtx::scope("engine", "options_engine_run_total");
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let ranked_pass = rank_compiled_tuples(&req, &prepared, dataset)?;
    let _result_scope = crate::nvtx::scope("engine", "options_result_finalize");
    let event_coverage = build_event_coverage_summary(&prepared);
    let split_invalid_reasons =
        build_split_invalid_reason_summary(&prepared.events, &prepared.baseline_candidate_cache);
    let coverage_adequacy = build_coverage_adequacy_summary(
        &prepared.diagnostics,
        &prepared.overlap_window,
        &event_coverage,
        &split_invalid_reasons,
        &prepared.baseline_candidate_cache.invalid_reason_counts(),
    );
    let ranked_results: Vec<OptionsExpressionTupleResult> = ranked_pass
        .ranked
        .into_iter()
        .map(|entry| entry.result)
        .collect();

    Ok(OptionsExpressionRunResult {
        diagnostics: prepared.diagnostics,
        backend_used: ranked_pass.backend_used.to_string(),
        backend_fallback_reason: ranked_pass.backend_fallback_reason,
        price_semantics: dataset.manifest.price_semantics,
        tuple_count: ranked_results.len(),
        staging: Some(prepared.staging),
        overlap_window: prepared.overlap_window,
        coverage_adequacy,
        split_invalid_reasons,
        ranked_results,
    })
}

pub fn analyze_options_expression_feasibility_blocking(
    req: OptionsExpressionRunRequest,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionFeasibilityReport, String> {
    let _nvtx_scope = crate::nvtx::scope("engine", "options_engine_feasibility_total");
    let prepare_start = Instant::now();
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let prepare_us = elapsed_us(prepare_start);
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let (resolved_gpu_device_id, gpu_device_resolution_us) = {
        let resolve_start = Instant::now();
        let device_id = current_preferred_cuda_device_id();
        (device_id, Some(elapsed_us(resolve_start)))
    };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let gpu_device_resolution_us: Option<u64> = None;
    let (cpu_ranking_us, cpu_ranking_ms) = {
        let _cpu_rank_scope = crate::nvtx::scope("engine", "options_feasibility_cpu_rank");
        let rank_start = Instant::now();
        let _ = rank_compiled_tuples_cpu(&req, &prepared, dataset);
        let cpu_ranking_us = elapsed_us(rank_start);
        (cpu_ranking_us, cpu_ranking_us / 1_000)
    };
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut gpu_probe_error: Option<String> = None;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let (gpu_plan_usage, gpu_plan_probe_us) = if let Some(device_id) = resolved_gpu_device_id {
        let _gpu_plan_scope = crate::nvtx::scope("engine", "options_feasibility_gpu_plan_probe");
        let probe_start = Instant::now();
        let usage = match inspect_gpu_plan_for_feasibility(
            device_id,
            &prepared.baseline_gpu_inputs,
            (prepared.diagnostics.holdout_events > 0).then_some(&prepared.conservative_gpu_inputs),
            (prepared.diagnostics.holdout_events > 0).then_some(&prepared.harsher_gpu_inputs),
        ) {
            Ok(usage) => Some(usage),
            Err(err) => {
                gpu_probe_error = Some(err);
                None
            }
        };
        (usage, Some(elapsed_us(probe_start)))
    } else {
        (None, None)
    };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let gpu_plan_probe_us: Option<u64> = None;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut gpu_ranking_error: Option<String> = None;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let (gpu_ranking_stats, gpu_ranking_us, gpu_ranking_ms) = {
        let _gpu_rank_scope = crate::nvtx::scope("engine", "options_feasibility_gpu_rank_probe");
        let stats = if gpu_probe_error.is_some()
            && !matches!(
                req.backend_preference,
                OptionsExpressionBackendPreference::GpuCompiledTensor
            ) {
            None
        } else {
            match gpu_scoring::gpu_ranking_stats_with_device(
                &req,
                &prepared,
                dataset,
                resolved_gpu_device_id,
            ) {
                Ok(stats) => stats,
                Err(err)
                    if matches!(
                        req.backend_preference,
                        OptionsExpressionBackendPreference::GpuCompiledTensor
                    ) =>
                {
                    return Err(err);
                }
                Err(err) => {
                    gpu_ranking_error = Some(err);
                    None
                }
            }
        };
        let us = stats.as_ref().map(|gpu| gpu.timing.total_us);
        let ms = us.map(|value| value / 1_000);
        (stats, us, ms)
    };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let gpu_ranking_us: Option<u64> = None;
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let gpu_ranking_ms: Option<u64> = None;
    let report_finalize_start = Instant::now();
    let candidate_slot_count = prepared.staging.candidate_slot_count;
    let compiled_candidate_count = prepared.staging.compiled_candidate_count;
    let invalid_slot_count = candidate_slot_count.saturating_sub(compiled_candidate_count);
    let baseline_exit_summary_count = prepared.staging.baseline_exit_summary_count;
    let conservative_exit_summary_count = prepared.staging.conservative_exit_summary_count;
    let harsher_exit_summary_count = prepared.staging.harsher_exit_summary_count;
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
    let harsher_compiled_exit_summary_count = prepared
        .harsher_exit_summary_cache
        .outcomes
        .iter()
        .filter(|entry| entry.is_some())
        .count();
    let mut timings = prepared.timings.clone();
    timings.prepare_us = prepare_us;
    timings.prepare_ms = prepare_us / 1_000;
    timings.gpu_device_resolution_us = gpu_device_resolution_us;
    timings.gpu_device_resolution_ms = gpu_device_resolution_us.map(|value| value / 1_000);
    timings.cpu_ranking_ms = cpu_ranking_ms;
    timings.cpu_ranking_us = cpu_ranking_us;
    timings.gpu_plan_probe_us = gpu_plan_probe_us;
    timings.gpu_plan_probe_ms = gpu_plan_probe_us.map(|value| value / 1_000);
    timings.gpu_ranking_ms = gpu_ranking_ms;
    timings.gpu_ranking_us = gpu_ranking_us;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        timings.gpu_tensor_score_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.tensor_score_us);
        timings.gpu_tensor_score_ms = timings.gpu_tensor_score_us.map(|value| value / 1_000);
        timings.gpu_exactify_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.exactify.total_us);
        timings.gpu_exactify_ms = timings.gpu_exactify_us.map(|value| value / 1_000);
        timings.gpu_exactify_summary_overwrite_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.exactify.summary_overwrite_us);
        timings.gpu_exactify_summary_overwrite_ms = timings
            .gpu_exactify_summary_overwrite_us
            .map(|value| value / 1_000);
        timings.gpu_exactify_enrichment_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.exactify.enrichment_us);
        timings.gpu_exactify_enrichment_ms = timings
            .gpu_exactify_enrichment_us
            .map(|value| value / 1_000);
        timings.gpu_exactify_extreme_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.exactify.extreme_us);
        timings.gpu_exactify_extreme_ms =
            timings.gpu_exactify_extreme_us.map(|value| value / 1_000);
        timings.gpu_sealed_stage_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.sealed_stage.total_us);
        timings.gpu_sealed_stage_ms = timings.gpu_sealed_stage_us.map(|value| value / 1_000);
        timings.gpu_sealed_non_extreme_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.sealed_stage.non_extreme_us);
        timings.gpu_sealed_non_extreme_ms =
            timings.gpu_sealed_non_extreme_us.map(|value| value / 1_000);
        timings.gpu_sealed_extreme_us = gpu_ranking_stats
            .as_ref()
            .map(|stats| stats.timing.sealed_stage.extreme_us);
        timings.gpu_sealed_extreme_ms = timings.gpu_sealed_extreme_us.map(|value| value / 1_000);
    }
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    {
        timings.gpu_tensor_score_us = None;
        timings.gpu_tensor_score_ms = None;
        timings.gpu_exactify_us = None;
        timings.gpu_exactify_ms = None;
        timings.gpu_exactify_summary_overwrite_us = None;
        timings.gpu_exactify_summary_overwrite_ms = None;
        timings.gpu_exactify_enrichment_us = None;
        timings.gpu_exactify_enrichment_ms = None;
        timings.gpu_exactify_extreme_us = None;
        timings.gpu_exactify_extreme_ms = None;
        timings.gpu_sealed_stage_us = None;
        timings.gpu_sealed_stage_ms = None;
        timings.gpu_sealed_non_extreme_us = None;
        timings.gpu_sealed_non_extreme_ms = None;
        timings.gpu_sealed_extreme_us = None;
        timings.gpu_sealed_extreme_ms = None;
    }
    timings.gpu_speedup_vs_cpu = gpu_ranking_us.and_then(|gpu_us| {
        if gpu_us == 0 {
            None
        } else {
            Some(cpu_ranking_us as f64 / gpu_us as f64)
        }
    });
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let mut cache_stats = prepared.cache_stats.clone();
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let cache_stats = prepared.cache_stats.clone();
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    {
        let mut observed_gpu_cache_usage = OptionsGpuRuntimeCacheUsage::default();
        if let Some(usage) = gpu_plan_usage {
            accumulate_cache_usage(&mut observed_gpu_cache_usage, usage);
        }
        if let Some(gpu_stats) = gpu_ranking_stats.as_ref() {
            accumulate_cache_usage(&mut observed_gpu_cache_usage, gpu_stats.cache_usage.clone());
        }
        if let Some(err) = gpu_probe_error.as_ref() {
            push_unique_gpu_note(
                &mut observed_gpu_cache_usage.gpu_plan_notes,
                format!("GPU planner probe failed before ranking: {err}"),
            );
        }
        if let Some(err) = gpu_ranking_error.as_ref() {
            push_unique_gpu_note(
                &mut observed_gpu_cache_usage.gpu_plan_notes,
                format!(
                    "GPU ranking attempt failed and feasibility fell back to CPU timing only: {err}"
                ),
            );
        }
        cache_stats.gpu_shared_input_cache_hits = observed_gpu_cache_usage.shared_input_cache_hits;
        cache_stats.gpu_shared_input_cache_misses =
            observed_gpu_cache_usage.shared_input_cache_misses;
        cache_stats.gpu_outcome_input_cache_hits =
            observed_gpu_cache_usage.outcome_input_cache_hits;
        cache_stats.gpu_outcome_input_cache_misses =
            observed_gpu_cache_usage.outcome_input_cache_misses;
        cache_stats.gpu_output_buffer_reuses = observed_gpu_cache_usage.output_buffer_reuses;
        cache_stats.gpu_output_buffer_allocations =
            observed_gpu_cache_usage.output_buffer_allocations;
        cache_stats.gpu_tuple_chunk_count = observed_gpu_cache_usage.tuple_chunk_count;
        cache_stats.gpu_tuple_chunk_size = observed_gpu_cache_usage.tuple_chunk_size;
        cache_stats.gpu_kernel_launch_count = observed_gpu_cache_usage.kernel_launch_count;
        cache_stats.gpu_tuple_budget_bytes = observed_gpu_cache_usage.gpu_tuple_budget_bytes;
        cache_stats.gpu_outcome_tensor_bytes = observed_gpu_cache_usage.gpu_outcome_tensor_bytes;
        cache_stats.gpu_shared_input_bytes_per_chunk =
            observed_gpu_cache_usage.gpu_shared_input_bytes_per_chunk;
        cache_stats.gpu_output_bytes_per_chunk =
            observed_gpu_cache_usage.gpu_output_bytes_per_chunk;
        cache_stats.gpu_total_device_bytes_per_chunk =
            observed_gpu_cache_usage.gpu_total_device_bytes_per_chunk;
        cache_stats.gpu_device_total_memory_bytes =
            observed_gpu_cache_usage.gpu_device_total_memory_bytes;
        cache_stats.gpu_device_free_memory_bytes =
            observed_gpu_cache_usage.gpu_device_free_memory_bytes;
        cache_stats.gpu_device_usable_memory_bytes =
            observed_gpu_cache_usage.gpu_device_usable_memory_bytes;
        cache_stats.gpu_memory_budget_source = observed_gpu_cache_usage.gpu_memory_budget_source;
        cache_stats.gpu_outcome_tensor_fits_in_memory =
            observed_gpu_cache_usage.gpu_outcome_tensor_fits_in_memory;
        cache_stats.gpu_chunk_plan_fits_in_memory =
            observed_gpu_cache_usage.gpu_chunk_plan_fits_in_memory;
        cache_stats.gpu_plan_notes = observed_gpu_cache_usage.gpu_plan_notes;
    }
    let split_invalid_reasons =
        build_split_invalid_reason_summary(&prepared.events, &prepared.baseline_candidate_cache);
    let invalid_reasons = prepared.baseline_candidate_cache.invalid_reason_counts();
    let event_coverage = build_event_coverage_summary(&prepared);
    let coverage_adequacy = build_coverage_adequacy_summary(
        &prepared.diagnostics,
        &prepared.overlap_window,
        &event_coverage,
        &split_invalid_reasons,
        &invalid_reasons,
    );
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let gpu_available = resolved_gpu_device_id.is_some();
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let gpu_available = false;
    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    let auto_backend_decision = {
        let mut decision = runtime_auto_backend_decision(&prepared, gpu_available);
        if let Some(err) = gpu_probe_error.as_ref().or(gpu_ranking_error.as_ref()) {
            decision.recommended_backend = "cpu_reference".to_string();
            decision.gpu_eligible = false;
            push_unique_gpu_note(
                &mut decision.reasons,
                format!("GPU scorer planning/runtime failed and CPU fallback was used: {err}"),
            );
        }
        decision
    };
    #[cfg(not(all(feature = "cuda", feature = "cuda-backtest-kernel")))]
    let auto_backend_decision = runtime_auto_backend_decision(&prepared, gpu_available);
    let best_available_backend =
        measured_best_available_backend(cpu_ranking_us, gpu_ranking_us).to_string();
    let projected_event_tuple_evaluations = projected_event_tuple_evaluations(&prepared);
    let report_finalize_us = elapsed_us(report_finalize_start);
    timings.report_finalize_us = report_finalize_us;
    timings.report_finalize_ms = report_finalize_us / 1_000;

    Ok(OptionsExpressionFeasibilityReport {
        diagnostics: prepared.diagnostics,
        price_semantics: dataset.manifest.price_semantics,
        fill_model: req.fill_model,
        best_available_backend,
        auto_backend_decision,
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
        harsher_exit_summary_count,
        harsher_compiled_exit_summary_count,
        harsher_exit_summary_density: ratio_or_zero(
            harsher_compiled_exit_summary_count,
            harsher_exit_summary_count,
        ),
        projected_event_tuple_evaluations,
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
        harsher_summary_bytes_estimate: prepared.harsher_exit_summary_cache.outcomes.len()
            * std::mem::size_of::<Option<TradeOutcome>>(),
        timings,
        event_coverage,
        overlap_window: prepared.overlap_window.clone(),
        split_invalid_reasons,
        coverage_adequacy,
        cache_stats,
        invalid_reasons,
        axis_coverage: prepared.baseline_candidate_cache.axis_coverage(),
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

fn elapsed_us(start: Instant) -> u64 {
    start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

pub fn inspect_options_expression_tuple_blocking(
    req: OptionsExpressionRunRequest,
    tuple_index: usize,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionTupleInspectionResult, String> {
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let ranked = rank_compiled_tuples(&req, &prepared, dataset)?.ranked;
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

pub fn inspect_options_expression_event_path_blocking(
    req: OptionsExpressionRunRequest,
    tuple_index: usize,
    event_index: usize,
    signal_manifest: &OptionsSignalEventManifestV1,
    dataset: &ImportedOptionsDataset,
) -> Result<OptionsExpressionEventPathInspectionResult, String> {
    let prepared = prepare_options_expression_run(&req, signal_manifest, dataset)?;
    let ranked = rank_compiled_tuples(&req, &prepared, dataset)?;
    let tuple_count = ranked.ranked.len();
    let selected = ranked.ranked.get(tuple_index).ok_or_else(|| {
        format!(
            "tuple_index {} is out of range for {} ranked tuple{}",
            tuple_index,
            tuple_count,
            if tuple_count == 1 { "" } else { "s" }
        )
    })?;
    let event = prepared.events.get(event_index).ok_or_else(|| {
        format!(
            "event_index {} is out of range for {} event{}",
            event_index,
            prepared.events.len(),
            if prepared.events.len() == 1 { "" } else { "s" }
        )
    })?;
    let index = IndexedOptionsDataset::from_dataset(dataset, &dataset.manifest.root_symbol);
    let row = build_event_detail_row(
        event_index,
        event,
        &selected.compiled,
        &index,
        &prepared.baseline_candidate_cache,
    );
    let points = if let Some(candidate) = prepared.baseline_candidate_cache.candidate(
        event_index,
        selected.compiled.delay_index,
        selected.compiled.dte_index,
        selected.compiled.moneyness_index,
    ) {
        let contract = index.contract(candidate.primary_leg.contract_index);
        let companion_contract = candidate
            .companion_leg
            .map(|leg| index.contract(leg.contract_index));
        let detail = evaluate_trade_detail(
            event,
            &selected.compiled.tuple,
            candidate,
            &index,
            index.dataset.manifest.price_semantics,
            AggregateStressMode::Baseline,
        );
        if let Some(detail) = detail {
            build_event_path_points(
                selected.compiled.tuple.structure_family,
                contract,
                companion_contract,
                detail.entry_index,
                candidate.companion_leg.map(|leg| leg.entry_index),
                detail.exit_index,
            )
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(OptionsExpressionEventPathInspectionResult {
        tuple_index,
        event_index,
        event_id: row.event_id.clone(),
        row,
        point_count: points.len(),
        points,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options_types::{
        ImportedOptionsDataset, OptionType, OptionsContractSeries, OptionsDatasetManifestV1,
        OptionsExpressionEventStatus, OptionsExpressionExitReason,
        OptionsExpressionValidationVerdict, OptionsFillModelFamily, OptionsMoneynessBucket,
        OptionsPriceRow, OptionsPriceSemantics, OptionsSignalEvent, OptionsSignalEventManifestV1,
        OptionsStaticFilterBundle, OptionsStructureFamily,
    };
    use crate::test_support::lock_test_env;
    use std::sync::atomic::AtomicUsize;

    fn next_isolated_cache_test_suffix() -> usize {
        static NEXT: AtomicUsize = AtomicUsize::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn isolate_cache_test_fixtures(
        req: &mut OptionsExpressionRunRequest,
        manifest: &mut OptionsSignalEventManifestV1,
        dataset: &mut ImportedOptionsDataset,
        label: &str,
    ) {
        let suffix = next_isolated_cache_test_suffix();
        let dataset_id = format!("spy_{label}_{suffix}");
        req.options_dataset_id = dataset_id.clone();
        req.parent_artifact_path = format!("C:/runs/{dataset_id}.json");
        manifest.manifest_id = format!("{dataset_id}_manifest");
        manifest.parent_artifact_id = format!("{dataset_id}_artifact");
        manifest.source_dataset_identity_hash = format!("{dataset_id}_source_dataset");
        manifest.source_content_fingerprint = format!("{dataset_id}_source_fingerprint");
        manifest.dataset_hash = format!("{dataset_id}_signal_dataset_hash");
        manifest.runtime_dataset_id = Some(dataset_id.clone());
        dataset.manifest.options_dataset_id = dataset_id.clone();
        dataset.manifest.dataset_path = format!("mem://{dataset_id}");
    }

    #[allow(dead_code)]
    fn metrics_with_values(
        valid_trades: usize,
        expectancy_per_trade: f64,
        expectancy_per_premium: f64,
        win_rate: f64,
    ) -> OptionsExpressionMetrics {
        OptionsExpressionMetrics {
            valid_trades,
            invalid_events: 0,
            invalid_rate: 0.0,
            expectancy_per_trade,
            expectancy_per_premium,
            mean_return_on_premium: expectancy_per_premium,
            return_volatility_on_premium: 0.0,
            median_return_on_premium: expectancy_per_premium,
            win_rate,
            total_pnl_per_contract: expectancy_per_trade * valid_trades as f64,
        }
    }

    fn metrics_with_invalid_rate(
        expectancy_per_premium: f64,
        invalid_rate: f64,
    ) -> OptionsExpressionMetrics {
        OptionsExpressionMetrics {
            invalid_rate,
            expectancy_per_trade: expectancy_per_premium * 100.0,
            ..metrics_with_values(
                4,
                expectancy_per_premium * 100.0,
                expectancy_per_premium,
                0.5,
            )
        }
    }

    #[cfg_attr(
        not(all(feature = "cuda", feature = "cuda-backtest-kernel")),
        allow(dead_code)
    )]
    fn assert_metrics_close(
        left: &OptionsExpressionMetrics,
        right: &OptionsExpressionMetrics,
        context: &str,
    ) {
        assert_eq!(
            left.valid_trades, right.valid_trades,
            "{context}: valid_trades"
        );
        assert_eq!(
            left.invalid_events, right.invalid_events,
            "{context}: invalid_events"
        );
        assert!(
            (left.invalid_rate - right.invalid_rate).abs() < 1e-12,
            "{context}: invalid_rate left={} right={}",
            left.invalid_rate,
            right.invalid_rate
        );
        assert!(
            (left.expectancy_per_trade - right.expectancy_per_trade).abs() < 1e-12,
            "{context}: expectancy_per_trade left={} right={}",
            left.expectancy_per_trade,
            right.expectancy_per_trade
        );
        assert!(
            (left.expectancy_per_premium - right.expectancy_per_premium).abs() < 1e-12,
            "{context}: expectancy_per_premium left={} right={}",
            left.expectancy_per_premium,
            right.expectancy_per_premium
        );
        assert!(
            (left.mean_return_on_premium - right.mean_return_on_premium).abs() < 1e-12,
            "{context}: mean_return_on_premium left={} right={}",
            left.mean_return_on_premium,
            right.mean_return_on_premium
        );
        assert!(
            (left.return_volatility_on_premium - right.return_volatility_on_premium).abs() < 1e-12,
            "{context}: return_volatility_on_premium left={} right={}",
            left.return_volatility_on_premium,
            right.return_volatility_on_premium
        );
        assert!(
            (left.median_return_on_premium - right.median_return_on_premium).abs() < 1e-12,
            "{context}: median_return_on_premium left={} right={}",
            left.median_return_on_premium,
            right.median_return_on_premium
        );
        assert!(
            (left.win_rate - right.win_rate).abs() < 1e-12,
            "{context}: win_rate left={} right={}",
            left.win_rate,
            right.win_rate
        );
        assert!(
            (left.total_pnl_per_contract - right.total_pnl_per_contract).abs() < 1e-12,
            "{context}: total_pnl_per_contract left={} right={}",
            left.total_pnl_per_contract,
            right.total_pnl_per_contract
        );
    }

    fn assert_trade_outcome_close(left: &TradeOutcome, right: &TradeOutcome, context: &str) {
        assert!(
            (left.pnl_per_contract - right.pnl_per_contract).abs() < 1e-12,
            "{context}: pnl_per_contract left={} right={}",
            left.pnl_per_contract,
            right.pnl_per_contract
        );
        assert!(
            (left.premium_paid - right.premium_paid).abs() < 1e-12,
            "{context}: premium_paid left={} right={}",
            left.premium_paid,
            right.premium_paid
        );
        assert!(
            (left.return_on_premium - right.return_on_premium).abs() < 1e-12,
            "{context}: return_on_premium left={} right={}",
            left.return_on_premium,
            right.return_on_premium
        );
    }

    #[test]
    fn compute_metrics_tracks_mean_and_return_on_premium_volatility() {
        let outcomes = vec![
            TradeOutcome {
                pnl_per_contract: 10.0,
                premium_paid: 10.0,
                return_on_premium: 1.0,
            },
            TradeOutcome {
                pnl_per_contract: -10.0,
                premium_paid: 10.0,
                return_on_premium: -1.0,
            },
            TradeOutcome {
                pnl_per_contract: 0.0,
                premium_paid: 5.0,
                return_on_premium: 0.0,
            },
        ];

        let metrics = compute_metrics(4, &outcomes);

        assert_eq!(metrics.valid_trades, 3);
        assert_eq!(metrics.invalid_events, 1);
        assert!((metrics.mean_return_on_premium - 0.0).abs() < 1e-12);
        assert!((metrics.return_volatility_on_premium - (2.0f64 / 3.0f64).sqrt()).abs() < 1e-12);
        assert_eq!(metrics.median_return_on_premium, 0.0);
    }

    #[test]
    fn compute_metrics_zero_valid_trades_zeroes_single_pass_expansion_metrics() {
        let metrics = compute_metrics(3, &[]);

        assert_eq!(metrics.valid_trades, 0);
        assert_eq!(metrics.invalid_events, 3);
        assert_eq!(metrics.mean_return_on_premium, 0.0);
        assert_eq!(metrics.return_volatility_on_premium, 0.0);
    }

    #[test]
    fn compute_metrics_single_trade_zeroes_return_on_premium_volatility() {
        let metrics = compute_metrics(
            1,
            &[TradeOutcome {
                pnl_per_contract: 12.0,
                premium_paid: 24.0,
                return_on_premium: 0.5,
            }],
        );

        assert_eq!(metrics.valid_trades, 1);
        assert_eq!(metrics.mean_return_on_premium, 0.5);
        assert_eq!(metrics.return_volatility_on_premium, 0.0);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn metrics_from_aggregate_reconstructs_mean_and_return_on_premium_volatility() {
        let outcomes = vec![
            TradeOutcome {
                pnl_per_contract: 10.0,
                premium_paid: 10.0,
                return_on_premium: 1.0,
            },
            TradeOutcome {
                pnl_per_contract: -10.0,
                premium_paid: 10.0,
                return_on_premium: -1.0,
            },
            TradeOutcome {
                pnl_per_contract: 0.0,
                premium_paid: 5.0,
                return_on_premium: 0.0,
            },
        ];
        let exact_metrics = compute_metrics(5, &outcomes);
        let aggregate_metrics = metrics_from_aggregate(TupleAggregateMetrics {
            valid_trades: 3,
            invalid_events: 2,
            win_count: 1,
            total_pnl_per_contract: 0.0,
            total_premium_paid: 25.0,
            sum_return_on_premium: 0.0,
            sumsq_return_on_premium: 2.0,
        });

        assert_eq!(aggregate_metrics.valid_trades, exact_metrics.valid_trades);
        assert_eq!(
            aggregate_metrics.invalid_events,
            exact_metrics.invalid_events
        );
        assert!(
            (aggregate_metrics.mean_return_on_premium - exact_metrics.mean_return_on_premium).abs()
                < 1e-12
        );
        assert!(
            (aggregate_metrics.return_volatility_on_premium
                - exact_metrics.return_volatility_on_premium)
                .abs()
                < 1e-12
        );
        assert_eq!(aggregate_metrics.median_return_on_premium, 0.0);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn exact_aggregate_for_split(
        tuple: &CompiledTupleSpec,
        events: &[OptionsSignalEvent],
        split: OptionsEvaluationSplit,
        candidate_cache: &StructuralCandidateCache,
        exit_summary_cache: &ExitSummaryCache,
    ) -> TupleAggregateMetrics {
        let mut split_event_count = 0usize;
        let mut valid_trades = 0usize;
        let mut win_count = 0usize;
        let mut total_pnl_per_contract = 0.0;
        let mut total_premium_paid = 0.0;
        let mut sum_return_on_premium = 0.0;
        let mut sumsq_return_on_premium = 0.0;

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
            if let Some(outcome) = exit_summary_cache.outcome(
                candidate_slot_index,
                tuple.hold_index,
                tuple.target_index,
                tuple.stop_index,
            ) {
                valid_trades += 1;
                total_pnl_per_contract += outcome.pnl_per_contract;
                total_premium_paid += outcome.premium_paid;
                sum_return_on_premium += outcome.return_on_premium;
                sumsq_return_on_premium += outcome.return_on_premium * outcome.return_on_premium;
                if outcome.pnl_per_contract > 0.0 {
                    win_count += 1;
                }
            }
        }

        TupleAggregateMetrics {
            valid_trades,
            invalid_events: split_event_count.saturating_sub(valid_trades),
            win_count,
            total_pnl_per_contract,
            total_premium_paid,
            sum_return_on_premium,
            sumsq_return_on_premium,
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    fn aggregate_metrics_from_gpu(
        aggregate: OptionsGpuTupleAggregate,
        split_event_count: usize,
    ) -> TupleAggregateMetrics {
        let valid_trades = aggregate.valid_trades as usize;
        TupleAggregateMetrics {
            valid_trades,
            invalid_events: split_event_count.saturating_sub(valid_trades),
            win_count: aggregate.wins as usize,
            total_pnl_per_contract: aggregate.total_pnl as f64,
            total_premium_paid: aggregate.total_premium as f64,
            sum_return_on_premium: aggregate.sum_return_on_premium as f64,
            sumsq_return_on_premium: aggregate.sumsq_return_on_premium as f64,
        }
    }

    fn test_dataset() -> ImportedOptionsDataset {
        ImportedOptionsDataset {
            manifest: OptionsDatasetManifestV1 {
                version: 1,
                options_dataset_id: "spy".to_string(),
                root_symbol: "SPY".to_string(),
                provider_name: "massive".to_string(),
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
                contract_reference_source: "massive_contract_reference".to_string(),
                contract_reference_as_of_utc: None,
                underlier_reference_source: "underlying_dataset".to_string(),
                dataset_path: "mem".to_string(),
                row_count: 16,
                contract_count: 4,
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
                OptionsContractSeries {
                    root_symbol: "SPY".to_string(),
                    contract_id: "SPY_C_2".to_string(),
                    option_type: OptionType::Call,
                    expiration_date: "2026-04-10".to_string(),
                    expiration_timestamp_ms: 1_700_086_400_000,
                    strike: 102.0,
                    multiplier: 100.0,
                    rows: vec![
                        OptionsPriceRow {
                            timestamp_ms: 1_700_000_060_000,
                            open: 0.6,
                            high: 0.7,
                            low: 0.5,
                            close: 0.6,
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
                            open: 0.6,
                            high: 0.95,
                            low: 0.55,
                            close: 0.85,
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
                    contract_id: "SPY_P_2".to_string(),
                    option_type: OptionType::Put,
                    expiration_date: "2026-04-10".to_string(),
                    expiration_timestamp_ms: 1_700_086_400_000,
                    strike: 98.0,
                    multiplier: 100.0,
                    rows: vec![
                        OptionsPriceRow {
                            timestamp_ms: 1_700_000_060_000,
                            open: 0.65,
                            high: 0.75,
                            low: 0.55,
                            close: 0.65,
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
                            open: 0.6,
                            high: 0.7,
                            low: 0.4,
                            close: 0.55,
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

    fn test_manifest_multi_session() -> OptionsSignalEventManifestV1 {
        let mut manifest = test_manifest();
        let template = manifest.events[0].clone();
        let make_event = |event_id: &str, earliest_tradable_timestamp: i64, session_id: &str| {
            OptionsSignalEvent {
                event_id: event_id.to_string(),
                decision_timestamp: earliest_tradable_timestamp - MINUTE_MS,
                earliest_tradable_timestamp,
                base_entry_timestamp: Some(earliest_tradable_timestamp),
                base_exit_timestamp: Some(earliest_tradable_timestamp + MINUTE_MS),
                session_id: Some(session_id.to_string()),
                fold: OptionsEvaluationSplit::Search,
                ..template.clone()
            }
        };
        manifest.events = vec![
            make_event("s1_a", 1_700_000_060_000, "2026-04-01"),
            make_event("s1_b", 1_700_000_120_000, "2026-04-01"),
            make_event("s2_a", 1_700_086_460_000, "2026-04-02"),
            make_event("s2_b", 1_700_086_520_000, "2026-04-02"),
            make_event("s3_a", 1_700_172_860_000, "2026-04-03"),
            make_event("s3_b", 1_700_172_920_000, "2026-04-03"),
        ];
        manifest
    }

    fn semantics_test_row() -> OptionsPriceRow {
        OptionsPriceRow {
            timestamp_ms: 1_700_000_060_000,
            open: 1.7,
            high: 2.4,
            low: 1.2,
            close: 1.8,
            underlying_price: 100.0,
            bid: Some(1.0),
            ask: Some(2.0),
            mid: Some(1.5),
            volume: Some(100.0),
            open_interest: Some(1000.0),
            quote_age_ms: Some(750),
            session_id: Some("2026-04-01".to_string()),
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn exactify_enriches_gpu_like_retained_rows_without_summary_overwrite() {
        let _guard = lock_test_env();
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };
        let dataset = test_dataset();
        let manifest = test_manifest();
        let prepared =
            prepare_options_expression_run(&req, &manifest, &dataset).expect("prepared run");
        let compiled = prepared
            .compiled_tuples
            .first()
            .cloned()
            .expect("compiled tuple");

        let selection_metrics = metrics_from_aggregate(exact_aggregate_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Search,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        ));
        let holdout_metrics = Some(metrics_from_aggregate(exact_aggregate_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        )));
        let stress_metrics = Some(metrics_from_aggregate(exact_aggregate_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.conservative_candidate_cache,
            &prepared.conservative_exit_summary_cache,
        )));
        let harsher_stress_metrics = Some(metrics_from_aggregate(exact_aggregate_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.harsher_candidate_cache,
            &prepared.harsher_exit_summary_cache,
        )));

        let mut ranked = vec![RankedCompiledTuple {
            compiled: compiled.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled.tuple.clone(),
                pre_sealed_rank: 0,
                sealed_finalist: false,
                sealed_final_rank: 0,
                selection_metrics,
                holdout_metrics,
                sealed_evaluation_metrics: None,
                stress_metrics,
                sealed_stress_metrics: None,
                harsher_stress_metrics,
                sealed_harsher_stress_metrics: None,
                extreme_stress_metrics: None,
                sealed_extreme_stress_metrics: None,
                validation_verdict: Default::default(),
                robust_score: 0.0,
            },
            retained_summary_state: RetainedSummaryState::gpu_initial(
                prepared.diagnostics.holdout_events,
            ),
        }];

        let timing = exactify_ranked_results(&prepared, &dataset, &mut ranked);
        let exact_selection = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Search,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        );
        let exact_holdout = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        );
        let exact_stress = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.conservative_candidate_cache,
            &prepared.conservative_exit_summary_cache,
        );
        let exact_harsher = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.harsher_candidate_cache,
            &prepared.harsher_exit_summary_cache,
        );
        let exact_extreme = evaluate_exact_stress_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &IndexedOptionsDataset::from_dataset(&dataset, &dataset.manifest.root_symbol),
            &prepared.baseline_candidate_cache,
            dataset.manifest.price_semantics,
            AggregateStressMode::Extreme,
        );

        assert_eq!(timing.summary_overwrite_us, 0);
        assert_metrics_close(
            &ranked[0].result.selection_metrics,
            &exact_selection,
            "selection",
        );
        assert_metrics_close(
            ranked[0].result.holdout_metrics.as_ref().expect("holdout"),
            &exact_holdout,
            "holdout",
        );
        assert_metrics_close(
            ranked[0].result.stress_metrics.as_ref().expect("stress"),
            &exact_stress,
            "stress",
        );
        assert_metrics_close(
            ranked[0]
                .result
                .harsher_stress_metrics
                .as_ref()
                .expect("harsher"),
            &exact_harsher,
            "harsher",
        );
        assert_metrics_close(
            ranked[0]
                .result
                .extreme_stress_metrics
                .as_ref()
                .expect("extreme"),
            &exact_extreme,
            "extreme",
        );
        assert_eq!(
            ranked[0].retained_summary_state.selection,
            RetainedMetricState::Exact
        );
        assert_eq!(
            ranked[0].retained_summary_state.holdout,
            RetainedMetricState::Exact
        );
        assert_eq!(
            ranked[0].retained_summary_state.stress,
            RetainedMetricState::Exact
        );
        assert_eq!(
            ranked[0].retained_summary_state.harsher,
            RetainedMetricState::Exact
        );
    }

    #[test]
    fn derived_mid_runtime_semantics_remain_aggregate_style() {
        let row = semantics_test_row();

        assert_eq!(
            option_entry_price_for_row(
                &row,
                OptionsPriceSemantics::DerivedMidSnapshot,
                OptionsFillModelFamily::AggregateResearch
            ),
            Some(1.8)
        );
        assert_eq!(
            option_exit_price_for_row(
                &row,
                OptionsPriceSemantics::DerivedMidSnapshot,
                OptionsFillModelFamily::AggregateResearch,
                AggregateStressMode::Conservative
            ),
            Some(1.2)
        );
        assert_eq!(
            option_path_range_for_row(
                &row,
                OptionsPriceSemantics::DerivedMidSnapshot,
                OptionsFillModelFamily::AggregateResearch,
                AggregateStressMode::Baseline
            ),
            Some((2.4, 1.2))
        );
    }

    #[test]
    fn quote_aware_runtime_semantics_use_quote_side_prices() {
        let row = semantics_test_row();

        assert_eq!(
            option_entry_price_for_row(
                &row,
                OptionsPriceSemantics::TopOfBookQuote,
                OptionsFillModelFamily::QuoteConservative
            ),
            Some(2.0)
        );
        assert_eq!(
            option_entry_price_for_row(
                &row,
                OptionsPriceSemantics::TopOfBookQuote,
                OptionsFillModelFamily::QuoteMid
            ),
            Some(1.5)
        );
        assert_eq!(
            option_exit_price_for_row(
                &row,
                OptionsPriceSemantics::TopOfBookQuote,
                OptionsFillModelFamily::QuoteConservative,
                AggregateStressMode::Extreme
            ),
            Some(1.0)
        );
        assert_eq!(
            option_path_range_for_row(
                &row,
                OptionsPriceSemantics::TopOfBookQuote,
                OptionsFillModelFamily::QuoteConservative,
                AggregateStressMode::Baseline
            ),
            Some((1.0, 1.0))
        );
    }

    #[test]
    fn options_expression_assign_splits_session_chronological_keeps_sessions_intact() {
        let manifest = test_manifest_multi_session();
        let events = assign_splits(
            &manifest,
            0.5,
            0.25,
            OptionsSplitAssignmentMode::SessionChronological,
        )
        .expect("events");

        let mut folds_by_session = BTreeMap::<String, Vec<OptionsEvaluationSplit>>::new();
        for event in events {
            let session_id = event
                .session_id
                .clone()
                .expect("session id should be set for test");
            folds_by_session
                .entry(session_id)
                .or_default()
                .push(event.fold);
        }

        assert_eq!(folds_by_session.len(), 3);
        let session1 = folds_by_session
            .get("2026-04-01")
            .expect("session 1 should be present");
        assert!(session1
            .iter()
            .all(|fold| *fold == OptionsEvaluationSplit::Search));
        let session2 = folds_by_session
            .get("2026-04-02")
            .expect("session 2 should be present");
        assert!(session2
            .iter()
            .all(|fold| *fold == OptionsEvaluationSplit::Holdout));
        let session3 = folds_by_session
            .get("2026-04-03")
            .expect("session 3 should be present");
        assert!(session3
            .iter()
            .all(|fold| *fold == OptionsEvaluationSplit::SealedTest));
    }

    #[test]
    fn options_expression_cpu_reference_scores_tuples() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
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
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };
        let dataset = test_dataset();
        let events = assign_splits(
            &test_manifest(),
            req.split_ratio_search,
            req.split_ratio_holdout,
            req.split_assignment_mode,
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
    fn evaluate_trade_summary_matches_detail_summary_across_structures_and_stress_modes() {
        let dataset = test_dataset();
        let manifest = test_manifest();
        let events = assign_splits(
            &manifest,
            0.5,
            0.25,
            OptionsSplitAssignmentMode::EventChronological,
        )
        .expect("events");
        let index = IndexedOptionsDataset::from_dataset(&dataset, &dataset.manifest.root_symbol);
        let structures = [
            OptionsStructureFamily::LongSingle,
            OptionsStructureFamily::DebitVertical,
            OptionsStructureFamily::LongPair,
        ];
        let stress_modes = [
            AggregateStressMode::Baseline,
            AggregateStressMode::Conservative,
            AggregateStressMode::Harsher,
            AggregateStressMode::Extreme,
        ];

        for structure_family in structures {
            for stress_mode in stress_modes {
                for (exit_template, target_return_buckets, stop_return_buckets, case_label) in [
                    (
                        OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                        vec![0.25],
                        vec![0.25],
                        "target_stop",
                    ),
                    (
                        OptionsExitTemplateFamily::FixedHold,
                        Vec::new(),
                        Vec::new(),
                        "fixed_hold",
                    ),
                ] {
                    let req = OptionsExpressionRunRequest {
                        parent_artifact_path: "C:/runs/a.json".to_string(),
                        parent_candidate_index: None,
                        options_dataset_id: "spy".to_string(),
                        structure_family,
                        backend_preference:
                            crate::options_types::OptionsExpressionBackendPreference::CpuReference,
                        delay_minutes: vec![0],
                        dte_buckets: vec![1],
                        moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
                        hold_minutes: vec![1],
                        target_return_buckets,
                        stop_return_buckets,
                        exit_template,
                        fill_model: OptionsFillModelFamily::AggregateResearch,
                        filters: OptionsStaticFilterBundle::default(),
                        top_k: 1,
                        min_valid_trades: None,
                        split_ratio_search: 0.5,
                        split_ratio_holdout: 0.25,
                        split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
                        sealed_finalist_count: 1,
                    };
                    let candidate_axis = CandidateAxis::from_request(&req).expect("candidate axis");
                    let exit_axis = ExitAxis::from_request(&req).expect("exit axis");
                    let candidate_cache = compile_structural_candidate_cache(
                        &events,
                        &dataset,
                        candidate_axis,
                        req.fill_model,
                        &req.filters,
                        stress_mode,
                    );
                    let mut matched = 0usize;
                    for (event_index, event) in events.iter().enumerate() {
                        let Some(candidate) = candidate_cache.candidate(event_index, 0, 0, 0)
                        else {
                            continue;
                        };
                        matched += 1;
                        for hold_minutes in exit_axis.hold_minutes.iter().copied() {
                            for target_return in exit_axis.target_values.iter().copied() {
                                for stop_return in exit_axis.stop_values.iter().copied() {
                                    let eval_tuple = OptionsTupleSpec {
                                        structure_family,
                                        delay_minutes: 0,
                                        dte_days: 1,
                                        moneyness_bucket: OptionsMoneynessBucket::Atm,
                                        hold_minutes,
                                        target_return,
                                        stop_return,
                                        exit_template,
                                        fill_model: req.fill_model,
                                    };
                                    let summary = evaluate_trade_summary(
                                        event,
                                        &eval_tuple,
                                        candidate,
                                        &index,
                                        dataset.manifest.price_semantics,
                                        stress_mode,
                                    );
                                    let detail = evaluate_trade_detail(
                                        event,
                                        &eval_tuple,
                                        candidate,
                                        &index,
                                        dataset.manifest.price_semantics,
                                        stress_mode,
                                    )
                                    .map(|outcome| outcome.summary());
                                    match (summary.as_ref(), detail.as_ref()) {
                                        (Some(summary), Some(detail)) => assert_trade_outcome_close(
                                            summary,
                                            detail,
                                            &format!(
                                                "{structure_family:?}/{stress_mode:?}/{case_label}/event={}",
                                                event.event_id
                                            ),
                                        ),
                                        (None, None) => {}
                                        (left, right) => panic!(
                                            "summary/detail mismatch for {structure_family:?}/{stress_mode:?}/{case_label}/event={}: summary_present={} detail_present={}",
                                            event.event_id,
                                            left.is_some(),
                                            right.is_some()
                                        ),
                                    }
                                }
                            }
                        }
                    }
                    assert!(
                        matched > 0,
                        "expected at least one valid candidate for {structure_family:?}/{stress_mode:?}/{case_label}"
                    );
                }
            }
        }
    }

    #[test]
    fn compile_exit_summary_cache_matches_per_summary_oracle_for_long_single() {
        let mut dataset = test_dataset();
        for contract in &mut dataset.contracts {
            let mut extra_row = contract.rows.last().expect("last row").clone();
            extra_row.timestamp_ms += MINUTE_MS;
            extra_row.open = extra_row.close;
            extra_row.high = extra_row.close + 0.05;
            extra_row.low = (extra_row.close - 0.05).max(0.0);
            extra_row.close += 0.02;
            extra_row.underlying_price += 0.25;
            contract.rows.push(extra_row);
        }
        dataset.manifest.row_count = dataset
            .contracts
            .iter()
            .map(|contract| contract.rows.len())
            .sum();
        let manifest = test_manifest();
        let events = assign_splits(
            &manifest,
            0.5,
            0.25,
            OptionsSplitAssignmentMode::EventChronological,
        )
        .expect("events");
        let index = IndexedOptionsDataset::from_dataset(&dataset, &dataset.manifest.root_symbol);
        let stress_modes = [
            AggregateStressMode::Baseline,
            AggregateStressMode::Conservative,
            AggregateStressMode::Harsher,
            AggregateStressMode::Extreme,
        ];

        for stress_mode in stress_modes {
            for (exit_template, target_return_buckets, stop_return_buckets, case_label) in [
                (
                    OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                    vec![0.25],
                    vec![0.25],
                    "target_stop",
                ),
                (
                    OptionsExitTemplateFamily::FixedHold,
                    Vec::new(),
                    Vec::new(),
                    "fixed_hold",
                ),
            ] {
                let req = OptionsExpressionRunRequest {
                    parent_artifact_path: "C:/runs/a.json".to_string(),
                    parent_candidate_index: None,
                    options_dataset_id: "spy".to_string(),
                    structure_family: OptionsStructureFamily::LongSingle,
                    backend_preference:
                        crate::options_types::OptionsExpressionBackendPreference::CpuReference,
                    delay_minutes: vec![0],
                    dte_buckets: vec![1],
                    moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
                    hold_minutes: vec![1, 2],
                    target_return_buckets,
                    stop_return_buckets,
                    exit_template,
                    fill_model: OptionsFillModelFamily::AggregateResearch,
                    filters: OptionsStaticFilterBundle::default(),
                    top_k: 1,
                    min_valid_trades: None,
                    split_ratio_search: 0.5,
                    split_ratio_holdout: 0.25,
                    split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
                    sealed_finalist_count: 1,
                };
                let candidate_axis = CandidateAxis::from_request(&req).expect("candidate axis");
                let exit_axis = ExitAxis::from_request(&req).expect("exit axis");
                let candidate_cache = compile_structural_candidate_cache(
                    &events,
                    &dataset,
                    candidate_axis,
                    req.fill_model,
                    &req.filters,
                    stress_mode,
                );
                let exit_cache = compile_exit_summary_cache(
                    &events,
                    &dataset,
                    &candidate_cache,
                    exit_axis.clone(),
                    stress_mode,
                );
                for (event_index, event) in events.iter().enumerate() {
                    let Some(candidate) = candidate_cache.candidate(event_index, 0, 0, 0) else {
                        continue;
                    };
                    let candidate_slot_index =
                        candidate_cache.candidate_slot_index(event_index, 0, 0, 0);
                    for (hold_index, hold_minutes) in
                        exit_axis.hold_minutes.iter().copied().enumerate()
                    {
                        for (target_index, target_return) in
                            exit_axis.target_values.iter().copied().enumerate()
                        {
                            for (stop_index, stop_return) in
                                exit_axis.stop_values.iter().copied().enumerate()
                            {
                                let eval_tuple = OptionsTupleSpec {
                                    structure_family: OptionsStructureFamily::LongSingle,
                                    delay_minutes: 0,
                                    dte_days: 1,
                                    moneyness_bucket: OptionsMoneynessBucket::Atm,
                                    hold_minutes,
                                    target_return,
                                    stop_return,
                                    exit_template,
                                    fill_model: req.fill_model,
                                };
                                let cached = exit_cache
                                    .outcome(
                                        candidate_slot_index,
                                        hold_index,
                                        target_index,
                                        stop_index,
                                    )
                                    .cloned();
                                let oracle = evaluate_trade_summary(
                                    event,
                                    &eval_tuple,
                                    candidate,
                                    &index,
                                    dataset.manifest.price_semantics,
                                    stress_mode,
                                );
                                match (cached.as_ref(), oracle.as_ref()) {
                                    (Some(cached), Some(oracle)) => assert_trade_outcome_close(
                                        cached,
                                        oracle,
                                        &format!(
                                            "{stress_mode:?}/{case_label}/event={}",
                                            event.event_id
                                        ),
                                    ),
                                    (None, None) => {}
                                    (left, right) => panic!(
                                        "cache/oracle mismatch for {stress_mode:?}/{case_label}/event={}: cached_present={} oracle_present={}",
                                        event.event_id,
                                        left.is_some(),
                                        right.is_some()
                                    ),
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn session_chronological_split_assignment_groups_whole_sessions() {
        let mut manifest = test_manifest();
        let template = manifest.events[0].clone();
        let make_event = |event_id: &str, earliest_tradable_timestamp: i64, session_id: &str| {
            let mut event = template.clone();
            event.event_id = event_id.to_string();
            event.decision_timestamp = earliest_tradable_timestamp - MINUTE_MS;
            event.earliest_tradable_timestamp = earliest_tradable_timestamp;
            event.base_entry_timestamp = Some(earliest_tradable_timestamp);
            event.base_exit_timestamp = Some(earliest_tradable_timestamp + MINUTE_MS);
            event.session_id = Some(session_id.to_string());
            event.fold = OptionsEvaluationSplit::Search;
            event
        };
        let day0 = 1_710_000_000_000i64;
        manifest.events = vec![
            make_event("day1_a", day0 + MINUTE_MS, "2026-04-01"),
            make_event("day1_b", day0 + 2 * MINUTE_MS, "2026-04-01"),
            make_event("day2_a", day0 + DAY_MS + MINUTE_MS, "2026-04-02"),
            make_event("day2_b", day0 + DAY_MS + 2 * MINUTE_MS, "2026-04-02"),
            make_event("day3_a", day0 + 2 * DAY_MS + MINUTE_MS, "2026-04-03"),
            make_event("day3_b", day0 + 2 * DAY_MS + 2 * MINUTE_MS, "2026-04-03"),
        ];

        let events = assign_splits(
            &manifest,
            0.5,
            0.25,
            OptionsSplitAssignmentMode::SessionChronological,
        )
        .expect("session chronological split assignment");
        let folds: Vec<_> = events
            .into_iter()
            .map(|event| (event.event_id, event.fold))
            .collect();
        assert_eq!(
            folds,
            vec![
                ("day1_a".to_string(), OptionsEvaluationSplit::Search),
                ("day1_b".to_string(), OptionsEvaluationSplit::Search),
                ("day2_a".to_string(), OptionsEvaluationSplit::Holdout),
                ("day2_b".to_string(), OptionsEvaluationSplit::Holdout),
                ("day3_a".to_string(), OptionsEvaluationSplit::SealedTest),
                ("day3_b".to_string(), OptionsEvaluationSplit::SealedTest),
            ]
        );
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn fused_gpu_search_holdout_stress_aggregates_match_exact_cpu_aggregates() {
        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let Some(device_id) = current_preferred_cuda_device_id() else {
            return;
        };

        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0, 1],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm, OptionsMoneynessBucket::Otm1],
            hold_minutes: vec![1, 2],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };
        let dataset = test_dataset();
        let manifest = test_manifest();
        let prepared =
            prepare_options_expression_run(&req, &manifest, &dataset).expect("prepared run");
        let search_end = prepared.diagnostics.search_events;
        let holdout_end = search_end + prepared.diagnostics.holdout_events;
        assert!(prepared.diagnostics.holdout_events > 0);

        let run = score_options_expression_tensors_gpu_search_holdout_stress(
            device_id,
            search_end,
            holdout_end,
            &prepared.baseline_gpu_inputs,
            &prepared.conservative_gpu_inputs,
            &prepared.harsher_gpu_inputs,
        )
        .expect("combined gpu scoring");
        assert_eq!(run.search_aggregates.len(), prepared.compiled_tuples.len());
        assert_eq!(run.holdout_aggregates.len(), prepared.compiled_tuples.len());
        assert_eq!(
            run.conservative_holdout_aggregates.len(),
            prepared.compiled_tuples.len()
        );
        assert_eq!(
            run.harsher_holdout_aggregates.len(),
            prepared.compiled_tuples.len()
        );
        assert_eq!(run.robust_scores.len(), prepared.compiled_tuples.len());

        for (tuple_index, compiled) in prepared.compiled_tuples.iter().enumerate() {
            let exact_search = exact_aggregate_for_split(
                compiled,
                &prepared.events,
                OptionsEvaluationSplit::Search,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            );
            let exact_holdout = exact_aggregate_for_split(
                compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.baseline_candidate_cache,
                &prepared.baseline_exit_summary_cache,
            );
            let exact_conservative = exact_aggregate_for_split(
                compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.conservative_candidate_cache,
                &prepared.conservative_exit_summary_cache,
            );
            let exact_harsher = exact_aggregate_for_split(
                compiled,
                &prepared.events,
                OptionsEvaluationSplit::Holdout,
                &prepared.harsher_candidate_cache,
                &prepared.harsher_exit_summary_cache,
            );
            let gpu_search =
                aggregate_metrics_from_gpu(run.search_aggregates[tuple_index], search_end);
            let gpu_holdout = aggregate_metrics_from_gpu(
                run.holdout_aggregates[tuple_index],
                prepared.diagnostics.holdout_events,
            );
            let gpu_conservative = aggregate_metrics_from_gpu(
                run.conservative_holdout_aggregates[tuple_index],
                prepared.diagnostics.holdout_events,
            );
            let gpu_harsher = aggregate_metrics_from_gpu(
                run.harsher_holdout_aggregates[tuple_index],
                prepared.diagnostics.holdout_events,
            );

            let lanes = [
                ("search", gpu_search, exact_search),
                ("holdout", gpu_holdout, exact_holdout),
                ("conservative", gpu_conservative, exact_conservative),
                ("harsher", gpu_harsher, exact_harsher),
            ];
            for (label, gpu, exact) in lanes {
                assert_eq!(gpu.valid_trades, exact.valid_trades, "{label} valid trades");
                assert_eq!(
                    gpu.invalid_events, exact.invalid_events,
                    "{label} invalid events"
                );
                assert_eq!(gpu.win_count, exact.win_count, "{label} wins");
                assert!(
                    (gpu.total_pnl_per_contract - exact.total_pnl_per_contract).abs() < 1e-6,
                    "{label} total pnl"
                );
                assert!(
                    (gpu.total_premium_paid - exact.total_premium_paid).abs() < 1e-6,
                    "{label} total premium"
                );
                assert!(
                    (gpu.sum_return_on_premium - exact.sum_return_on_premium).abs() < 1e-6,
                    "{label} sum return on premium"
                );
                assert!(
                    (gpu.sumsq_return_on_premium - exact.sumsq_return_on_premium).abs() < 1e-6,
                    "{label} sumsq return on premium"
                );
            }

            let exact_search_metrics = metrics_from_aggregate(exact_search);
            let exact_holdout_metrics = metrics_from_aggregate(exact_holdout);
            let exact_conservative_metrics = metrics_from_aggregate(exact_conservative);
            let exact_harsher_metrics = metrics_from_aggregate(exact_harsher);
            let expected_robust = robust_score(
                &exact_search_metrics,
                Some(&exact_holdout_metrics),
                None,
                Some(&exact_conservative_metrics),
                Some(&exact_harsher_metrics),
                None,
            );
            assert!(
                (run.robust_scores[tuple_index] - expected_robust).abs() < 1e-9,
                "robust score"
            );
        }
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn gpu_combined_ranking_matches_cpu_topk_on_small_surface() {
        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let Some(device_id) = current_preferred_cuda_device_id() else {
            return;
        };
        std::env::remove_var("VECTORGRID_OPTIONS_GPU_DISABLE_COMBINED_SCORING");

        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0, 1],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm, OptionsMoneynessBucket::Otm1],
            hold_minutes: vec![1, 2],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 5,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };
        let dataset = test_dataset();
        let manifest = test_manifest();
        let prepared =
            prepare_options_expression_run(&req, &manifest, &dataset).expect("prepared run");
        let gpu_ranked = gpu_scoring::rank_compiled_tuples_gpu_with_device(
            &req,
            &prepared,
            &dataset,
            Some(device_id),
        )
        .expect("gpu ranking")
        .expect("gpu ranked results");
        let cpu_ranked = rank_compiled_tuples_cpu(&req, &prepared, &dataset);

        assert_eq!(gpu_ranked.ranked_results.len(), cpu_ranked.len());
        for (gpu_row, cpu_row) in gpu_ranked.ranked_results.iter().zip(cpu_ranked.iter()) {
            assert_eq!(gpu_row.result.tuple, cpu_row.result.tuple);
            assert_eq!(
                gpu_row.result.pre_sealed_rank,
                cpu_row.result.pre_sealed_rank
            );
            assert_eq!(
                gpu_row.result.sealed_finalist,
                cpu_row.result.sealed_finalist
            );
            assert_eq!(
                gpu_row.result.sealed_final_rank,
                cpu_row.result.sealed_final_rank
            );
            assert!(
                (gpu_row.result.robust_score - cpu_row.result.robust_score).abs() < 1e-9,
                "robust score parity"
            );
        }
    }

    #[test]
    fn options_expression_run_populates_staging_stats() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
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
        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
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
        assert_eq!(report.axis_coverage.delay_minutes.len(), 1);
        assert_eq!(report.axis_coverage.dte_days.len(), 2);
        assert_eq!(
            report.axis_coverage.dte_days[0].slot_count
                + report.axis_coverage.dte_days[1].slot_count,
            report.candidate_slot_count
        );
        assert_eq!(report.axis_coverage.dte_days[0].dte_days, 1);
        assert_eq!(report.axis_coverage.dte_days[1].dte_days, 9);
        assert_eq!(report.axis_coverage.dte_days[0].valid_slot_count, 3);
        assert_eq!(report.axis_coverage.dte_days[1].valid_slot_count, 0);
        assert!(report.axis_coverage.dte_days[1]
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
        assert_eq!(report.overlap_window.total_events_in_dataset_window, 3);
        assert_eq!(report.overlap_window.search_events_in_dataset_window, 1);
        assert_eq!(report.overlap_window.holdout_events_in_dataset_window, 1);
        assert_eq!(
            report.overlap_window.sealed_test_events_in_dataset_window,
            1
        );
        assert_eq!(report.overlap_window.total_overlap_rate, 1.0);
        assert_eq!(report.overlap_window.search_overlap_rate, 1.0);
        assert_eq!(
            report.coverage_adequacy.verdict,
            OptionsExpressionCoverageAdequacyVerdict::MixedFragile
        );
        assert!(!report.coverage_adequacy.non_promotable);
        assert_eq!(report.coverage_adequacy.search_candidate_coverage_rate, 1.0);
        assert_eq!(
            report
                .coverage_adequacy
                .search_baseline_outcome_coverage_rate,
            1.0
        );
        assert_eq!(
            report.coverage_adequacy.dominant_invalid_reason,
            Some(OptionsCandidateInvalidReason::NoExpirationInBucket)
        );
        assert!(report
            .coverage_adequacy
            .reasons
            .iter()
            .any(|reason| reason.contains("Dominant structural invalid reason")));
        assert_eq!(report.split_invalid_reasons.search_invalid_reasons.len(), 1);
        assert_eq!(
            report.split_invalid_reasons.search_invalid_reasons[0].reason,
            OptionsCandidateInvalidReason::NoExpirationInBucket
        );
        assert_eq!(
            report.split_invalid_reasons.search_invalid_reasons[0].count,
            1
        );
        assert_eq!(
            report.split_invalid_reasons.holdout_invalid_reasons.len(),
            1
        );
        assert_eq!(
            report
                .split_invalid_reasons
                .sealed_test_invalid_reasons
                .len(),
            1
        );
        assert!(report.structural_cache_bytes_estimate > 0);
        assert!(report.baseline_summary_bytes_estimate > 0);
        assert!(report.conservative_summary_bytes_estimate > 0);
    }

    #[test]
    fn options_expression_feasibility_report_flags_missing_search_overlap_as_non_promotable() {
        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };
        let mut manifest = test_manifest();
        for event in &mut manifest.events {
            event.decision_timestamp += 14 * DAY_MS;
            event.earliest_tradable_timestamp += 14 * DAY_MS;
            event.base_entry_timestamp =
                event.base_entry_timestamp.map(|value| value + 14 * DAY_MS);
            event.base_exit_timestamp = event.base_exit_timestamp.map(|value| value + 14 * DAY_MS);
        }

        let report =
            analyze_options_expression_feasibility_blocking(req, &manifest, &test_dataset())
                .expect("report");

        assert_eq!(report.overlap_window.total_events_in_dataset_window, 0);
        assert_eq!(report.overlap_window.search_events_in_dataset_window, 0);
        assert_eq!(report.event_coverage.search_events_with_candidates, 0);
        assert_eq!(
            report.coverage_adequacy.verdict,
            OptionsExpressionCoverageAdequacyVerdict::SearchOverlapMissing
        );
        assert!(report.coverage_adequacy.non_promotable);
        assert!(report.coverage_adequacy.reasons.iter().any(|reason| {
            reason.contains("No search events fall inside the imported options dataset window")
        }));
    }

    #[test]
    fn options_expression_prepare_cache_distinguishes_shifted_manifest_events() {
        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let mut req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };
        let mut baseline_manifest = test_manifest();
        let mut dataset = test_dataset();
        isolate_cache_test_fixtures(
            &mut req,
            &mut baseline_manifest,
            &mut dataset,
            "prepare_cache_shifted_manifest",
        );

        let baseline = analyze_options_expression_feasibility_blocking(
            req.clone(),
            &baseline_manifest,
            &dataset,
        )
        .expect("baseline report");
        assert_eq!(baseline.event_coverage.search_events_with_candidates, 1);
        assert_eq!(baseline.cache_stats.structural_cache_hits, 0);

        let mut shifted_manifest = baseline_manifest.clone();
        for event in &mut shifted_manifest.events {
            event.decision_timestamp += 14 * DAY_MS;
            event.earliest_tradable_timestamp += 14 * DAY_MS;
            event.base_entry_timestamp =
                event.base_entry_timestamp.map(|value| value + 14 * DAY_MS);
            event.base_exit_timestamp = event.base_exit_timestamp.map(|value| value + 14 * DAY_MS);
        }

        let shifted =
            analyze_options_expression_feasibility_blocking(req, &shifted_manifest, &dataset)
                .expect("shifted report");

        assert_eq!(shifted.overlap_window.search_events_in_dataset_window, 0);
        assert_eq!(shifted.event_coverage.search_events_with_candidates, 0);
        assert_eq!(shifted.cache_stats.structural_cache_hits, 0);
        assert!(shifted.cache_stats.structural_cache_misses >= 2);
    }

    #[test]
    fn options_expression_feasibility_report_tracks_cache_reuse_on_warm_preview() {
        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let mut req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0, 5],
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };
        let mut manifest = test_manifest();
        let mut dataset = test_dataset();
        isolate_cache_test_fixtures(
            &mut req,
            &mut manifest,
            &mut dataset,
            "warm_preview_cache_reuse",
        );

        let cold =
            analyze_options_expression_feasibility_blocking(req.clone(), &manifest, &dataset)
                .expect("cold report");
        let warm = analyze_options_expression_feasibility_blocking(req, &manifest, &dataset)
            .expect("warm report");

        assert_eq!(cold.cache_stats.structural_cache_hits, 0);
        assert_eq!(cold.cache_stats.summary_cache_hits, 0);
        assert!(cold.cache_stats.structural_cache_misses >= 2);
        assert!(cold.cache_stats.summary_cache_misses >= 2);
        assert!(warm.cache_stats.structural_cache_hits >= 2);
        assert!(warm.cache_stats.summary_cache_hits >= 2);
        assert_eq!(warm.cache_stats.structural_cache_misses, 0);
        assert_eq!(warm.cache_stats.summary_cache_misses, 0);
    }

    #[cfg(all(feature = "cuda", feature = "cuda-backtest-kernel"))]
    #[test]
    fn options_expression_feasibility_report_records_gpu_planner_failure_notes() {
        struct ForcedGpuPlanReset;

        impl Drop for ForcedGpuPlanReset {
            fn drop(&mut self) {
                set_forced_options_expression_cuda_device_id_for_test(None);
                set_forced_options_expression_gpu_plan_error_for_test(None);
            }
        }

        let _env = lock_test_env();
        reset_options_expression_prepare_caches();
        let _reset = ForcedGpuPlanReset;
        set_forced_options_expression_cuda_device_id_for_test(Some(Some(0)));
        set_forced_options_expression_gpu_plan_error_for_test(Some(
            "synthetic GPU planner failure".to_string(),
        ));

        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };

        let report =
            analyze_options_expression_feasibility_blocking(req, &test_manifest(), &test_dataset())
                .expect("report");

        assert_eq!(
            report.auto_backend_decision.recommended_backend,
            "cpu_reference"
        );
        assert!(!report.auto_backend_decision.gpu_eligible);
        assert!(report
            .auto_backend_decision
            .reasons
            .iter()
            .any(|entry| entry.contains("synthetic GPU planner failure")));
        assert!(report
            .cache_stats
            .gpu_plan_notes
            .iter()
            .any(|entry| entry.contains("synthetic GPU planner failure")));
    }

    #[test]
    fn auto_backend_decision_prefers_cpu_for_small_workloads() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
        };

        let prepared = prepare_options_expression_run(&req, &test_manifest(), &test_dataset())
            .expect("prepared");
        let decision = evaluate_auto_backend_decision(&prepared, true, Some(4.0));

        assert!(!decision.gpu_eligible);
        assert_eq!(decision.recommended_backend, "cpu_reference");
        assert!(decision
            .reasons
            .iter()
            .any(|entry| entry.contains("Compiled tuple count")));
        assert!(decision
            .reasons
            .iter()
            .any(|entry| entry.contains("Search-event count")));
    }

    #[test]
    fn auto_backend_decision_promotes_gpu_for_dense_large_workloads() {
        let template_manifest = test_manifest();
        let template_event = template_manifest.events[0].clone();
        let mut dense_manifest = template_manifest.clone();
        dense_manifest.events = (0..120)
            .map(|idx| {
                let mut event = template_event.clone();
                event.event_id = format!("dense_{idx}");
                event.decision_timestamp = template_event.earliest_tradable_timestamp - MINUTE_MS;
                event.earliest_tradable_timestamp = template_event.earliest_tradable_timestamp;
                event.base_entry_timestamp = Some(template_event.earliest_tradable_timestamp);
                event.base_exit_timestamp =
                    Some(template_event.earliest_tradable_timestamp + MINUTE_MS);
                event.side = if idx % 2 == 0 {
                    SignalSide::Bullish
                } else {
                    SignalSide::Bearish
                };
                event
            })
            .collect();

        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: (1..=32).collect(),
            target_return_buckets: vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
            stop_return_buckets: vec![0.1, 0.2, 0.3, 0.4],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 25,
            min_valid_trades: None,
            split_ratio_search: 0.6,
            split_ratio_holdout: 0.2,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 5,
        };

        let prepared = prepare_options_expression_run(&req, &dense_manifest, &test_dataset())
            .expect("prepared");
        let decision = evaluate_auto_backend_decision(&prepared, true, Some(4.0));

        assert!(decision.gpu_eligible);
        assert_eq!(decision.recommended_backend, "gpu_compiled_tensor_hybrid");
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn auto_backend_decision_promotes_gpu_for_medium_dense_workloads() {
        let template_manifest = test_manifest();
        let template_event = template_manifest.events[0].clone();
        let mut manifest = template_manifest.clone();
        manifest.events = (0..20)
            .map(|idx| {
                let mut event = template_event.clone();
                event.event_id = format!("medium_{idx}");
                event.side = if idx % 2 == 0 {
                    SignalSide::Bullish
                } else {
                    SignalSide::Bearish
                };
                event.fold = if idx < 12 {
                    OptionsEvaluationSplit::Search
                } else if idx < 16 {
                    OptionsEvaluationSplit::Holdout
                } else {
                    OptionsEvaluationSplit::SealedTest
                };
                event
            })
            .collect();

        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0, 1, 2, 3],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: (1..=32).collect(),
            target_return_buckets: vec![0.1, 0.2, 0.3, 0.4],
            stop_return_buckets: vec![0.1, 0.2, 0.3, 0.4],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 25,
            min_valid_trades: None,
            split_ratio_search: 0.6,
            split_ratio_holdout: 0.2,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 5,
        };

        let prepared =
            prepare_options_expression_run(&req, &manifest, &test_dataset()).expect("prepared");
        let decision = evaluate_auto_backend_decision(&prepared, true, None);

        assert!(decision.gpu_eligible);
        assert_eq!(decision.recommended_backend, "gpu_compiled_tensor_hybrid");
        assert!(decision.reasons.is_empty());
    }

    #[test]
    fn runtime_auto_backend_decision_stays_gpu_eligible_even_if_measured_speedup_is_below_promotion_gate(
    ) {
        let template_manifest = test_manifest();
        let template_event = template_manifest.events[0].clone();
        let mut manifest = template_manifest.clone();
        manifest.events = (0..20)
            .map(|idx| {
                let mut event = template_event.clone();
                event.event_id = format!("medium_{idx}");
                event.side = if idx % 2 == 0 {
                    SignalSide::Bullish
                } else {
                    SignalSide::Bearish
                };
                event.fold = if idx < 12 {
                    OptionsEvaluationSplit::Search
                } else if idx < 16 {
                    OptionsEvaluationSplit::Holdout
                } else {
                    OptionsEvaluationSplit::SealedTest
                };
                event
            })
            .collect();

        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0, 1, 2, 3],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: (1..=32).collect(),
            target_return_buckets: vec![0.1, 0.2, 0.3, 0.4],
            stop_return_buckets: vec![0.1, 0.2, 0.3, 0.4],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 25,
            min_valid_trades: None,
            split_ratio_search: 0.6,
            split_ratio_holdout: 0.2,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 5,
        };

        let prepared =
            prepare_options_expression_run(&req, &manifest, &test_dataset()).expect("prepared");
        let runtime_decision = runtime_auto_backend_decision(&prepared, true);
        let promotion_decision = evaluate_auto_backend_decision(&prepared, true, Some(1.23));

        assert!(runtime_decision.gpu_eligible);
        assert_eq!(
            runtime_decision.recommended_backend,
            "gpu_compiled_tensor_hybrid"
        );
        assert!(!promotion_decision.gpu_eligible);
        assert_eq!(promotion_decision.recommended_backend, "cpu_reference");
        assert!(promotion_decision
            .reasons
            .iter()
            .any(|entry| entry.contains("Measured GPU hybrid speedup")));
    }

    #[test]
    fn robust_score_excludes_sealed_metrics_from_tuple_ranking() {
        let base = OptionsExpressionMetrics {
            valid_trades: 4,
            invalid_events: 0,
            invalid_rate: 0.0,
            expectancy_per_trade: 10.0,
            expectancy_per_premium: 0.20,
            mean_return_on_premium: 0.15,
            return_volatility_on_premium: 0.05,
            median_return_on_premium: 0.10,
            win_rate: 0.5,
            total_pnl_per_contract: 40.0,
        };
        let holdout = OptionsExpressionMetrics {
            expectancy_per_premium: 0.10,
            ..base.clone()
        };
        let stress = OptionsExpressionMetrics {
            expectancy_per_premium: 0.05,
            ..base.clone()
        };
        let sealed_low = OptionsExpressionMetrics {
            expectancy_per_premium: -0.50,
            ..base.clone()
        };
        let sealed_high = OptionsExpressionMetrics {
            expectancy_per_premium: 0.90,
            ..base
        };

        let left = robust_score(
            &holdout,
            Some(&holdout),
            Some(&sealed_low),
            Some(&stress),
            Some(&stress),
            Some(&stress),
        );
        let right = robust_score(
            &holdout,
            Some(&holdout),
            Some(&sealed_high),
            Some(&stress),
            Some(&stress),
            Some(&stress),
        );

        assert_eq!(left, right);
    }

    #[test]
    fn robust_score_penalizes_worse_holdout_invalid_rate() {
        let search = metrics_with_invalid_rate(0.20, 0.0);
        let holdout_good = metrics_with_invalid_rate(0.10, 0.0);
        let holdout_bad = metrics_with_invalid_rate(0.10, 0.50);

        let good = robust_score(&search, Some(&holdout_good), None, None, None, None);
        let bad = robust_score(&search, Some(&holdout_bad), None, None, None, None);

        assert!(good > bad);
    }

    #[test]
    fn robust_score_penalizes_worse_stress_invalid_rate() {
        let search = metrics_with_invalid_rate(0.20, 0.0);
        let stress_good = metrics_with_invalid_rate(0.08, 0.0);
        let stress_bad = metrics_with_invalid_rate(0.08, 0.50);

        let good = robust_score(&search, None, None, Some(&stress_good), None, None);
        let bad = robust_score(&search, None, None, Some(&stress_bad), None, None);

        assert!(good > bad);
    }

    #[test]
    fn robust_score_penalizes_worse_harsher_stress_invalid_rate() {
        let search = metrics_with_invalid_rate(0.20, 0.0);
        let stress = metrics_with_invalid_rate(0.08, 0.0);
        let harsher_good = metrics_with_invalid_rate(0.04, 0.0);
        let harsher_bad = metrics_with_invalid_rate(0.04, 0.50);

        let good = robust_score(
            &search,
            None,
            None,
            Some(&stress),
            Some(&harsher_good),
            None,
        );
        let bad = robust_score(&search, None, None, Some(&stress), Some(&harsher_bad), None);

        assert!(good > bad);
    }

    #[test]
    fn robust_score_missing_holdout_matches_explicit_search_equivalent_holdout() {
        let search = metrics_with_invalid_rate(0.20, 0.10);

        let implicit = robust_score(&search, None, None, None, None, None);
        let explicit = robust_score(&search, Some(&search), None, None, None, None);

        assert_eq!(implicit, explicit);
    }

    #[test]
    fn robust_score_missing_harsher_matches_explicit_stress_equivalent_harsher() {
        let search = metrics_with_invalid_rate(0.20, 0.10);
        let stress = metrics_with_invalid_rate(0.08, 0.25);

        let implicit = robust_score(&search, None, None, Some(&stress), None, None);
        let explicit = robust_score(&search, None, None, Some(&stress), Some(&stress), None);

        assert_eq!(implicit, explicit);
    }

    #[test]
    fn robust_score_equal_lane_invalid_rates_reduce_to_weighted_expectancy() {
        let common_invalid_rate = 0.25;
        let search = metrics_with_invalid_rate(0.20, common_invalid_rate);
        let holdout = metrics_with_invalid_rate(0.10, common_invalid_rate);
        let stress = metrics_with_invalid_rate(0.05, common_invalid_rate);
        let harsher = metrics_with_invalid_rate(0.03, common_invalid_rate);
        let extreme = metrics_with_invalid_rate(-0.02, common_invalid_rate);

        let score = robust_score(
            &search,
            Some(&holdout),
            None,
            Some(&stress),
            Some(&harsher),
            Some(&extreme),
        );
        let expected = (1.0 - common_invalid_rate)
            * (0.45 * search.expectancy_per_premium
                + 0.25 * holdout.expectancy_per_premium
                + 0.15 * stress.expectancy_per_premium
                + 0.10 * harsher.expectancy_per_premium
                + 0.05 * extreme.expectancy_per_premium);

        assert!((score - expected).abs() < 1e-12);
    }

    #[test]
    fn sealed_final_rank_uses_sealed_metrics_ordering() {
        let metrics = |valid_trades, expectancy_per_trade, expectancy_per_premium, win_rate| {
            OptionsExpressionMetrics {
                valid_trades,
                invalid_events: 0,
                invalid_rate: 0.0,
                expectancy_per_trade,
                expectancy_per_premium,
                mean_return_on_premium: expectancy_per_premium,
                return_volatility_on_premium: 0.0,
                median_return_on_premium: expectancy_per_premium,
                win_rate,
                total_pnl_per_contract: expectancy_per_trade * valid_trades as f64,
            }
        };
        let make_compiled = |dte_days| CompiledTupleSpec {
            tuple: OptionsTupleSpec {
                structure_family: OptionsStructureFamily::LongSingle,
                delay_minutes: 0,
                dte_days,
                moneyness_bucket: OptionsMoneynessBucket::Atm,
                hold_minutes: 15,
                target_return: Some(0.25),
                stop_return: Some(0.25),
                exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                fill_model: OptionsFillModelFamily::AggregateResearch,
            },
            delay_index: 0,
            dte_index: 0,
            moneyness_index: 0,
            hold_index: 0,
            target_index: 0,
            stop_index: 0,
        };
        let left_compiled = make_compiled(1);
        let right_compiled = make_compiled(3);
        let mut ranked = vec![
            RankedCompiledTuple {
                compiled: left_compiled.clone(),
                result: OptionsExpressionTupleResult {
                    tuple: left_compiled.tuple.clone(),
                    pre_sealed_rank: 1,
                    sealed_finalist: true,
                    sealed_final_rank: 0,
                    selection_metrics: metrics(8, 0.30, 0.20, 0.60),
                    holdout_metrics: None,
                    sealed_evaluation_metrics: Some(metrics(4, 0.05, 0.04, 0.50)),
                    stress_metrics: None,
                    sealed_stress_metrics: Some(metrics(4, 0.04, 0.03, 0.50)),
                    harsher_stress_metrics: None,
                    sealed_harsher_stress_metrics: Some(metrics(4, 0.03, 0.02, 0.50)),
                    extreme_stress_metrics: None,
                    sealed_extreme_stress_metrics: Some(metrics(4, 0.02, 0.01, 0.50)),
                    validation_verdict: Default::default(),
                    robust_score: 0.30,
                },
                retained_summary_state: RetainedSummaryState::cpu_initial(0),
            },
            RankedCompiledTuple {
                compiled: right_compiled.clone(),
                result: OptionsExpressionTupleResult {
                    tuple: right_compiled.tuple.clone(),
                    pre_sealed_rank: 2,
                    sealed_finalist: true,
                    sealed_final_rank: 0,
                    selection_metrics: metrics(7, 0.20, 0.10, 0.55),
                    holdout_metrics: None,
                    sealed_evaluation_metrics: Some(metrics(4, 0.11, 0.08, 0.50)),
                    stress_metrics: None,
                    sealed_stress_metrics: Some(metrics(4, 0.09, 0.07, 0.50)),
                    harsher_stress_metrics: None,
                    sealed_harsher_stress_metrics: Some(metrics(4, 0.08, 0.06, 0.50)),
                    extreme_stress_metrics: None,
                    sealed_extreme_stress_metrics: Some(metrics(4, 0.07, 0.05, 0.50)),
                    validation_verdict: Default::default(),
                    robust_score: 0.20,
                },
                retained_summary_state: RetainedSummaryState::cpu_initial(0),
            },
        ];

        assign_sealed_final_ranks(&mut ranked);

        assert_eq!(ranked[0].result.pre_sealed_rank, 1);
        assert_eq!(ranked[0].result.sealed_final_rank, 2);
        assert_eq!(ranked[1].result.pre_sealed_rank, 2);
        assert_eq!(ranked[1].result.sealed_final_rank, 1);
    }

    #[test]
    fn refresh_validation_verdicts_requires_supported_validation_lanes() {
        let metrics = |valid_trades,
                       invalid_events,
                       expectancy_per_trade,
                       expectancy_per_premium,
                       win_rate| {
            let total_events = valid_trades + invalid_events;
            OptionsExpressionMetrics {
                valid_trades,
                invalid_events,
                invalid_rate: if total_events == 0 {
                    0.0
                } else {
                    invalid_events as f64 / total_events as f64
                },
                expectancy_per_trade,
                expectancy_per_premium,
                mean_return_on_premium: expectancy_per_premium,
                return_volatility_on_premium: 0.0,
                median_return_on_premium: expectancy_per_premium,
                win_rate,
                total_pnl_per_contract: expectancy_per_trade * valid_trades as f64,
            }
        };
        let compiled = CompiledTupleSpec {
            tuple: OptionsTupleSpec {
                structure_family: OptionsStructureFamily::LongSingle,
                delay_minutes: 0,
                dte_days: 1,
                moneyness_bucket: OptionsMoneynessBucket::Atm,
                hold_minutes: 15,
                target_return: Some(0.25),
                stop_return: Some(0.25),
                exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
                fill_model: OptionsFillModelFamily::AggregateResearch,
            },
            delay_index: 0,
            dte_index: 0,
            moneyness_index: 0,
            hold_index: 0,
            target_index: 0,
            stop_index: 0,
        };
        let mut ranked = vec![RankedCompiledTuple {
            compiled: compiled.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled.tuple.clone(),
                pre_sealed_rank: 1,
                sealed_finalist: true,
                sealed_final_rank: 1,
                selection_metrics: metrics(8, 1, 0.30, 0.20, 0.60),
                holdout_metrics: Some(metrics(0, 2, 0.0, 0.09, 0.0)),
                sealed_evaluation_metrics: Some(metrics(0, 1, 0.0, 0.08, 0.0)),
                stress_metrics: Some(metrics(2, 0, 0.04, 0.04, 0.50)),
                sealed_stress_metrics: None,
                harsher_stress_metrics: None,
                sealed_harsher_stress_metrics: None,
                extreme_stress_metrics: None,
                sealed_extreme_stress_metrics: None,
                validation_verdict: OptionsExpressionValidationVerdict::SealedFinalist,
                robust_score: 0.18,
            },
            retained_summary_state: RetainedSummaryState::cpu_initial(2),
        }];

        refresh_validation_verdicts(&mut ranked);
        assert_eq!(
            ranked[0].result.validation_verdict,
            OptionsExpressionValidationVerdict::SearchOnly
        );

        ranked[0].result.sealed_finalist = false;
        ranked[0].result.sealed_evaluation_metrics = None;
        ranked[0].result.holdout_metrics = Some(metrics(2, 0, 0.09, 0.09, 0.50));
        ranked[0].result.stress_metrics = Some(metrics(2, 0, 0.08, 0.08, 0.50));

        refresh_validation_verdicts(&mut ranked);
        assert_eq!(
            ranked[0].result.validation_verdict,
            OptionsExpressionValidationVerdict::HoldoutSupported
        );
    }

    #[test]
    fn options_expression_only_sealed_evaluates_configured_finalists() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm, OptionsMoneynessBucket::Otm1],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 2,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };

        let result =
            run_options_expression_blocking(req, &test_manifest(), &test_dataset()).expect("run");
        assert_eq!(result.ranked_results.len(), 2);
        assert_eq!(
            result
                .ranked_results
                .iter()
                .filter(|row| row.sealed_finalist)
                .count(),
            1
        );
        assert_eq!(
            result
                .ranked_results
                .iter()
                .filter(|row| row.sealed_evaluation_metrics.is_some())
                .count(),
            1
        );
        assert_eq!(result.ranked_results[0].pre_sealed_rank, 1);
        assert!(result.ranked_results[0].sealed_finalist);
        assert!(result.ranked_results[0].sealed_evaluation_metrics.is_some());
        assert!(result.ranked_results[0].sealed_stress_metrics.is_some());
        assert_eq!(result.ranked_results[1].pre_sealed_rank, 2);
        assert!(!result.ranked_results[1].sealed_finalist);
        assert!(result.ranked_results[1].sealed_evaluation_metrics.is_none());
        assert!(result.ranked_results[1].sealed_stress_metrics.is_none());
    }

    #[test]
    fn exactify_ranked_results_cpu_rows_fill_missing_harsher_metrics() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };
        let manifest = test_manifest();
        let dataset = test_dataset();
        let prepared =
            prepare_options_expression_run(&req, &manifest, &dataset).expect("prepared run");
        let compiled = prepared
            .compiled_tuples
            .first()
            .cloned()
            .expect("compiled tuple");
        let holdout_events = prepared.diagnostics.holdout_events;
        assert!(holdout_events > 0);

        let selection_metrics = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Search,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        );
        let holdout_metrics = Some(evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        ));
        let stress_metrics = Some(evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.conservative_candidate_cache,
            &prepared.conservative_exit_summary_cache,
        ));
        let expected_harsher = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.harsher_candidate_cache,
            &prepared.harsher_exit_summary_cache,
        );

        let mut ranked = vec![RankedCompiledTuple {
            compiled: compiled.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled.tuple.clone(),
                pre_sealed_rank: 1,
                sealed_finalist: false,
                sealed_final_rank: 0,
                selection_metrics: selection_metrics.clone(),
                holdout_metrics: holdout_metrics.clone(),
                sealed_evaluation_metrics: None,
                stress_metrics: stress_metrics.clone(),
                sealed_stress_metrics: None,
                harsher_stress_metrics: None,
                sealed_harsher_stress_metrics: None,
                extreme_stress_metrics: None,
                sealed_extreme_stress_metrics: None,
                validation_verdict: Default::default(),
                robust_score: robust_score(
                    &selection_metrics,
                    holdout_metrics.as_ref(),
                    None,
                    stress_metrics.as_ref(),
                    None,
                    None,
                ),
            },
            retained_summary_state: RetainedSummaryState::cpu_initial(holdout_events),
        }];

        let timing = exactify_ranked_results(&prepared, &dataset, &mut ranked);
        assert!(timing.total_us >= timing.summary_overwrite_us);
        assert!(timing.total_us >= timing.extreme_us);
        assert_eq!(
            ranked[0].retained_summary_state.harsher,
            RetainedMetricState::Exact
        );
        let harsher = ranked[0]
            .result
            .harsher_stress_metrics
            .as_ref()
            .expect("harsher metrics");
        assert_eq!(harsher.valid_trades, expected_harsher.valid_trades);
        assert_eq!(harsher.invalid_events, expected_harsher.invalid_events);
        assert!(
            (harsher.expectancy_per_premium - expected_harsher.expectancy_per_premium).abs()
                < 1e-12
        );
        assert!(
            (harsher.median_return_on_premium - expected_harsher.median_return_on_premium).abs()
                < 1e-12
        );
        assert!(ranked[0].result.extreme_stress_metrics.is_some());
    }

    #[test]
    fn exactify_ranked_results_reuses_gpu_summary_metrics_and_only_enriches_medians() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };
        let manifest = test_manifest();
        let dataset = test_dataset();
        let prepared =
            prepare_options_expression_run(&req, &manifest, &dataset).expect("prepared run");
        let compiled = prepared
            .compiled_tuples
            .first()
            .cloned()
            .expect("compiled tuple");
        let holdout_events = prepared.diagnostics.holdout_events;
        assert!(holdout_events > 0);

        let selection_exact = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Search,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        );
        let holdout_exact = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.baseline_candidate_cache,
            &prepared.baseline_exit_summary_cache,
        );
        let stress_exact = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.conservative_candidate_cache,
            &prepared.conservative_exit_summary_cache,
        );
        let harsher_exact = evaluate_compiled_tuple_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &prepared.harsher_candidate_cache,
            &prepared.harsher_exit_summary_cache,
        );
        let expected_extreme = evaluate_exact_stress_for_split(
            &compiled,
            &prepared.events,
            OptionsEvaluationSplit::Holdout,
            &IndexedOptionsDataset::from_dataset(&dataset, &dataset.manifest.root_symbol),
            &prepared.baseline_candidate_cache,
            dataset.manifest.price_semantics,
            AggregateStressMode::Extreme,
        );

        let mut selection_gpu = selection_exact.clone();
        selection_gpu.expectancy_per_premium += 1.0;
        selection_gpu.median_return_on_premium = 0.0;
        let mut holdout_gpu = holdout_exact.clone();
        holdout_gpu.expectancy_per_premium += 0.5;
        holdout_gpu.median_return_on_premium = 0.0;
        let mut stress_gpu = stress_exact.clone();
        stress_gpu.expectancy_per_premium += 0.25;
        stress_gpu.median_return_on_premium = 0.0;
        let mut harsher_gpu = harsher_exact.clone();
        harsher_gpu.expectancy_per_premium += 0.125;
        harsher_gpu.median_return_on_premium = 0.0;

        let mut ranked = vec![RankedCompiledTuple {
            compiled: compiled.clone(),
            result: OptionsExpressionTupleResult {
                tuple: compiled.tuple.clone(),
                pre_sealed_rank: 1,
                sealed_finalist: false,
                sealed_final_rank: 0,
                selection_metrics: selection_gpu.clone(),
                holdout_metrics: Some(holdout_gpu.clone()),
                sealed_evaluation_metrics: None,
                stress_metrics: Some(stress_gpu.clone()),
                sealed_stress_metrics: None,
                harsher_stress_metrics: Some(harsher_gpu.clone()),
                sealed_harsher_stress_metrics: None,
                extreme_stress_metrics: None,
                sealed_extreme_stress_metrics: None,
                validation_verdict: Default::default(),
                robust_score: robust_score(
                    &selection_gpu,
                    Some(&holdout_gpu),
                    None,
                    Some(&stress_gpu),
                    Some(&harsher_gpu),
                    None,
                ),
            },
            retained_summary_state: RetainedSummaryState::gpu_initial(holdout_events),
        }];

        let timing = exactify_ranked_results(&prepared, &dataset, &mut ranked);
        assert!(timing.total_us >= timing.enrichment_us);
        assert!(timing.total_us >= timing.extreme_us);
        assert_eq!(
            ranked[0].retained_summary_state.selection,
            RetainedMetricState::Exact
        );
        assert_eq!(
            ranked[0].retained_summary_state.holdout,
            RetainedMetricState::Exact
        );
        assert_eq!(
            ranked[0].retained_summary_state.stress,
            RetainedMetricState::Exact
        );
        assert_eq!(
            ranked[0].retained_summary_state.harsher,
            RetainedMetricState::Exact
        );

        assert!(
            (ranked[0].result.selection_metrics.expectancy_per_premium
                - selection_gpu.expectancy_per_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0]
                .result
                .holdout_metrics
                .as_ref()
                .expect("holdout")
                .expectancy_per_premium
                - holdout_gpu.expectancy_per_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0]
                .result
                .stress_metrics
                .as_ref()
                .expect("stress")
                .expectancy_per_premium
                - stress_gpu.expectancy_per_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0]
                .result
                .harsher_stress_metrics
                .as_ref()
                .expect("harsher")
                .expectancy_per_premium
                - harsher_gpu.expectancy_per_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0].result.selection_metrics.median_return_on_premium
                - selection_exact.median_return_on_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0]
                .result
                .holdout_metrics
                .as_ref()
                .expect("holdout")
                .median_return_on_premium
                - holdout_exact.median_return_on_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0]
                .result
                .stress_metrics
                .as_ref()
                .expect("stress")
                .median_return_on_premium
                - stress_exact.median_return_on_premium)
                .abs()
                < 1e-12
        );
        assert!(
            (ranked[0]
                .result
                .harsher_stress_metrics
                .as_ref()
                .expect("harsher")
                .median_return_on_premium
                - harsher_exact.median_return_on_premium)
                .abs()
                < 1e-12
        );

        let extreme = ranked[0]
            .result
            .extreme_stress_metrics
            .as_ref()
            .expect("extreme metrics");
        assert_eq!(extreme.valid_trades, expected_extreme.valid_trades);
        assert_eq!(extreme.invalid_events, expected_extreme.invalid_events);
        assert!(
            (extreme.expectancy_per_premium - expected_extreme.expectancy_per_premium).abs()
                < 1e-12
        );
        assert!(
            (extreme.median_return_on_premium - expected_extreme.median_return_on_premium).abs()
                < 1e-12
        );
    }

    #[test]
    fn tuple_inspection_returns_event_rows_for_ranked_tuple() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongSingle,
            backend_preference: crate::options_types::OptionsExpressionBackendPreference::Auto,
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
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 2,
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
        assert_eq!(inspection.rows[0].candidate_invalid_reason, None);
        assert_eq!(inspection.rows[0].contract_id.as_deref(), Some("SPY_C_1"));
        assert_eq!(inspection.rows[0].entry_lag_minutes, Some(0));
        assert_eq!(inspection.rows[0].minutes_to_expiry_at_entry, Some(1439));
        assert_eq!(inspection.rows[0].realized_hold_minutes, Some(1));
        assert!(inspection.rows[0].path_option_high.unwrap_or_default() >= 1.25);
        assert!(inspection.rows[0].path_option_low.unwrap_or_default() <= 1.0);
        assert!(inspection.rows[0].path_underlier_high.unwrap_or_default() >= 100.5);
        assert!(inspection.rows[0].path_underlier_low.unwrap_or_default() <= 100.0);
        assert_eq!(
            inspection.rows[0].exit_reason,
            Some(OptionsExpressionExitReason::TargetHit)
        );
        if let Some(missing_row) = inspection
            .rows
            .iter()
            .find(|row| row.status == OptionsExpressionEventStatus::CandidateMissing)
        {
            assert!(missing_row.candidate_invalid_reason.is_some());
        }
        assert!(inspection.rows.iter().all(|row| !row.event_id.is_empty()));
    }

    #[test]
    fn tuple_inspection_returns_companion_leg_for_debit_vertical() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::DebitVertical,
            backend_preference:
                crate::options_types::OptionsExpressionBackendPreference::CpuReference,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };

        let inspection =
            inspect_options_expression_tuple_blocking(req, 0, &test_manifest(), &test_dataset())
                .expect("inspection");
        let first_row = inspection.rows.first().expect("first row");
        assert_eq!(first_row.contract_id.as_deref(), Some("SPY_C_1"));
        assert_eq!(first_row.companion_contract_id.as_deref(), Some("SPY_C_2"));
        assert_eq!(first_row.companion_strike, Some(102.0));
        assert!(first_row.companion_entry_option_price.is_some());
    }

    #[test]
    fn tuple_inspection_returns_companion_leg_for_long_pair() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongPair,
            backend_preference:
                crate::options_types::OptionsExpressionBackendPreference::CpuReference,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };

        let inspection =
            inspect_options_expression_tuple_blocking(req, 0, &test_manifest(), &test_dataset())
                .expect("inspection");
        let first_row = inspection.rows.first().expect("first row");
        assert_eq!(first_row.contract_id.as_deref(), Some("SPY_C_1"));
        assert_eq!(first_row.companion_contract_id.as_deref(), Some("SPY_C_2"));
        assert_eq!(first_row.companion_strike, Some(102.0));
        assert_eq!(first_row.entry_option_price, Some(1.6));
        assert_eq!(first_row.companion_entry_option_price, Some(0.6));
        assert!(
            (first_row.return_on_premium.unwrap_or_default() - 0.25).abs() < 1e-9,
            "unexpected return_on_premium: {:?}",
            first_row.return_on_premium
        );
    }

    #[test]
    fn event_path_replay_returns_net_position_points_for_debit_vertical() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::DebitVertical,
            backend_preference:
                crate::options_types::OptionsExpressionBackendPreference::CpuReference,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };

        let inspection = inspect_options_expression_event_path_blocking(
            req,
            0,
            0,
            &test_manifest(),
            &test_dataset(),
        )
        .expect("path inspection");

        assert!(!inspection.points.is_empty());
        assert!((inspection.points[0].option_close - 0.40).abs() < 1e-6);
        assert!(inspection.row.companion_contract_id.is_some());
    }

    #[test]
    fn event_path_replay_returns_net_position_points_for_long_pair() {
        let req = OptionsExpressionRunRequest {
            parent_artifact_path: "C:/runs/a.json".to_string(),
            parent_candidate_index: None,
            options_dataset_id: "spy".to_string(),
            structure_family: OptionsStructureFamily::LongPair,
            backend_preference:
                crate::options_types::OptionsExpressionBackendPreference::CpuReference,
            delay_minutes: vec![0],
            dte_buckets: vec![1],
            moneyness_buckets: vec![OptionsMoneynessBucket::Atm],
            hold_minutes: vec![1],
            target_return_buckets: vec![0.25],
            stop_return_buckets: vec![0.25],
            exit_template: OptionsExitTemplateFamily::FixedHoldOrTargetStop,
            fill_model: OptionsFillModelFamily::AggregateResearch,
            filters: OptionsStaticFilterBundle::default(),
            top_k: 1,
            min_valid_trades: None,
            split_ratio_search: 0.5,
            split_ratio_holdout: 0.25,
            split_assignment_mode: OptionsSplitAssignmentMode::EventChronological,
            sealed_finalist_count: 1,
        };

        let inspection = inspect_options_expression_event_path_blocking(
            req,
            0,
            0,
            &test_manifest(),
            &test_dataset(),
        )
        .expect("path inspection");

        assert_eq!(inspection.point_count, 2);
        assert!((inspection.points[0].option_open - 1.60).abs() < 1e-6);
        assert!((inspection.points[0].option_high - 1.80).abs() < 1e-6);
        assert!((inspection.points[0].option_low - 1.40).abs() < 1e-6);
        assert!((inspection.points[0].option_close - 1.60).abs() < 1e-6);
        assert!((inspection.points[1].option_high - 2.35).abs() < 1e-6);
        assert!((inspection.points[1].option_low - 1.50).abs() < 1e-6);
        assert!((inspection.points[1].option_close - 2.15).abs() < 1e-6);
        assert!(inspection.row.companion_contract_id.is_some());
    }
}
