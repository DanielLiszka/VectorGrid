//! Strategy-local temporal validation contract for VCET.
//!
//! This module intentionally contains only deterministic, side-effect-free planning and
//! validation helpers. Callers remain responsible for executing VCET and for persisting the
//! returned plan identity and dataset observation alongside their artifacts.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter, Write as _};

pub const VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION: u16 = 1;
pub const VCET_TEMPORAL_VALIDATION_PLAN_HASH_CONTRACT: &str =
    "vectorgrid.vcet.temporal_validation_plan.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalFoldModeV1 {
    Anchored,
    Rolling,
}

/// Position handling is explicit because silently carrying a position across a scored boundary
/// makes train, validation, and final-test results dependent on an earlier phase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalPositionBoundaryPolicyV1 {
    /// Start every scored interval flat and discard any still-open position at its end.
    ResetFlatDiscardOpen,
    /// Start flat and force-liquidate any open position on the final scored bar.
    ResetFlatForceLiquidate,
    /// Start flat and reject the interval unless the strategy is naturally flat at its end.
    ResetFlatRequireClosed,
}

/// Closed bounds over candle timestamps expressed as Unix epoch milliseconds in UTC.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalUtcBoundsV1 {
    pub start_utc_ms_inclusive: i64,
    pub end_utc_ms_inclusive: i64,
}

/// Closed bounds over canonical session identifiers.
///
/// Each session identifier is the Unix epoch millisecond timestamp of that session's canonical
/// open in UTC. Callers decide which calendar maps candles to sessions, but the calendar identity
/// is sealed into the plan and must match every dataset observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalSessionBoundsV1 {
    pub calendar_id: String,
    pub first_session_open_utc_ms_inclusive: i64,
    pub last_session_open_utc_ms_inclusive: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalRequestedBoundsV1 {
    pub utc: VcetTemporalUtcBoundsV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<VcetTemporalSessionBoundsV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalMinimumSupportV1 {
    pub min_folds: usize,
    pub min_train_bars: usize,
    pub min_validation_bars: usize,
    pub min_final_test_bars: usize,
    pub min_train_trades_per_fold: u64,
    pub min_validation_trades_per_fold: u64,
    pub min_final_test_trades: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalValidationPlanV1 {
    pub schema_version: u16,
    pub requested_bounds: VcetTemporalRequestedBoundsV1,
    pub fold_mode: VcetTemporalFoldModeV1,
    pub train_bars: usize,
    pub validation_bars: usize,
    pub final_test_bars: usize,
    pub step_bars: usize,
    pub purge_bars: usize,
    pub embargo_bars: usize,
    pub warmup_bars: usize,
    pub position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1,
    pub minimum_support: VcetTemporalMinimumSupportV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPlanHashInputV1 {
    pub hash_contract: String,
    pub schema_version: u16,
    pub plan: VcetTemporalValidationPlanV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalPlanIdentityV1 {
    pub schema_version: u16,
    pub plan_id: String,
    pub plan_sha256: String,
}

/// Half-open absolute bar-index range into the caller-provided dataset.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalBarRangeV1 {
    pub start_index: usize,
    pub end_index_exclusive: usize,
}

impl VcetTemporalBarRangeV1 {
    pub fn len(self) -> usize {
        self.end_index_exclusive.saturating_sub(self.start_index)
    }

    pub fn is_empty(self) -> bool {
        self.start_index >= self.end_index_exclusive
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalDatasetObservationV1 {
    pub dataset_id: String,
    pub bar_count: usize,
    pub observed_utc: VcetTemporalUtcBoundsV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_session: Option<VcetTemporalSessionBoundsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_session_count: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalDatasetSliceV1 {
    pub observation: VcetTemporalDatasetObservationV1,
    pub requested_bounds: VcetTemporalRequestedBoundsV1,
    pub selected_bars: VcetTemporalBarRangeV1,
    pub selected_utc: VcetTemporalUtcBoundsV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_session: Option<VcetTemporalSessionBoundsV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalEvaluationSliceV1 {
    /// History available to initialize indicators. These bars are never scored.
    pub warmup_bars: VcetTemporalBarRangeV1,
    pub scored_bars: VcetTemporalBarRangeV1,
    pub scored_utc: VcetTemporalUtcBoundsV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scored_session: Option<VcetTemporalSessionBoundsV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFoldV1 {
    pub fold_index: usize,
    pub train: VcetTemporalEvaluationSliceV1,
    pub purge_before_validation: VcetTemporalBarRangeV1,
    pub validation: VcetTemporalEvaluationSliceV1,
    pub embargo_after_validation: VcetTemporalBarRangeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalLayoutV1 {
    pub identity: VcetTemporalPlanIdentityV1,
    pub dataset: VcetTemporalDatasetSliceV1,
    pub folds: Vec<VcetTemporalFoldV1>,
    /// Unscored alignment remainder after the last validation embargo and before final-test purge.
    pub sealed_selection_remainder: VcetTemporalBarRangeV1,
    pub purge_before_final_test: VcetTemporalBarRangeV1,
    pub final_test: VcetTemporalEvaluationSliceV1,
    pub position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalFoldTradeSupportV1 {
    pub fold_index: usize,
    pub train_trades: u64,
    pub validation_trades: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalTradeSupportV1 {
    pub folds: Vec<VcetTemporalFoldTradeSupportV1>,
    pub final_test_trades: u64,
}

/// Borrowed canonical session mapping aligned one-for-one with candle timestamps.
#[derive(Clone, Copy, Debug)]
pub struct VcetTemporalSessionSeriesRef<'a> {
    pub calendar_id: &'a str,
    pub session_open_utc_ms: &'a [i64],
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VcetTemporalErrorCode {
    UnsupportedSchemaVersion,
    InvalidPlan,
    InvalidBounds,
    ArithmeticOverflow,
    EmptyDataset,
    DatasetShapeMismatch,
    NonMonotonicTimestamps,
    MissingSessionSeries,
    SessionCalendarMismatch,
    CoverageInadequate,
    InsufficientBars,
    InsufficientFolds,
    IntervalOverlap,
    CrossDatasetMisalignment,
    IdentityMismatch,
    TradeSupportInadequate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VcetTemporalValidationError {
    pub code: VcetTemporalErrorCode,
    pub message: String,
}

impl Display for VcetTemporalValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for VcetTemporalValidationError {}

pub type VcetTemporalResult<T> = Result<T, VcetTemporalValidationError>;

fn temporal_error(
    code: VcetTemporalErrorCode,
    message: impl Into<String>,
) -> VcetTemporalValidationError {
    VcetTemporalValidationError {
        code,
        message: message.into(),
    }
}

fn checked_add(left: usize, right: usize, field: &str) -> VcetTemporalResult<usize> {
    left.checked_add(right).ok_or_else(|| {
        temporal_error(
            VcetTemporalErrorCode::ArithmeticOverflow,
            format!("VCET temporal plan arithmetic overflow while computing {field}"),
        )
    })
}

fn checked_mul(left: usize, right: usize, field: &str) -> VcetTemporalResult<usize> {
    left.checked_mul(right).ok_or_else(|| {
        temporal_error(
            VcetTemporalErrorCode::ArithmeticOverflow,
            format!("VCET temporal plan arithmetic overflow while computing {field}"),
        )
    })
}

fn validate_utc_bounds(bounds: VcetTemporalUtcBoundsV1, field: &str) -> VcetTemporalResult<()> {
    if bounds.start_utc_ms_inclusive > bounds.end_utc_ms_inclusive {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            format!(
                "{field} start_utc_ms_inclusive must be <= end_utc_ms_inclusive ({} > {})",
                bounds.start_utc_ms_inclusive, bounds.end_utc_ms_inclusive
            ),
        ));
    }
    Ok(())
}

fn validate_session_bounds(
    bounds: &VcetTemporalSessionBoundsV1,
    field: &str,
) -> VcetTemporalResult<()> {
    if bounds.calendar_id.trim().is_empty() {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            format!("{field}.calendar_id must not be empty"),
        ));
    }
    if bounds.first_session_open_utc_ms_inclusive > bounds.last_session_open_utc_ms_inclusive {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            format!(
                "{field} first session must be <= last session ({} > {})",
                bounds.first_session_open_utc_ms_inclusive,
                bounds.last_session_open_utc_ms_inclusive
            ),
        ));
    }
    Ok(())
}

pub fn validate_vcet_temporal_plan(plan: &VcetTemporalValidationPlanV1) -> VcetTemporalResult<()> {
    if plan.schema_version != VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION {
        return Err(temporal_error(
            VcetTemporalErrorCode::UnsupportedSchemaVersion,
            format!(
                "unsupported VCET temporal validation schema version {}; expected {}",
                plan.schema_version, VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION
            ),
        ));
    }
    validate_utc_bounds(plan.requested_bounds.utc, "requested_bounds.utc")?;
    if let Some(bounds) = plan.requested_bounds.session.as_ref() {
        validate_session_bounds(bounds, "requested_bounds.session")?;
    }

    for (name, value) in [
        ("train_bars", plan.train_bars),
        ("validation_bars", plan.validation_bars),
        ("final_test_bars", plan.final_test_bars),
        ("step_bars", plan.step_bars),
        ("minimum_support.min_folds", plan.minimum_support.min_folds),
        (
            "minimum_support.min_train_bars",
            plan.minimum_support.min_train_bars,
        ),
        (
            "minimum_support.min_validation_bars",
            plan.minimum_support.min_validation_bars,
        ),
        (
            "minimum_support.min_final_test_bars",
            plan.minimum_support.min_final_test_bars,
        ),
    ] {
        if value == 0 {
            return Err(temporal_error(
                VcetTemporalErrorCode::InvalidPlan,
                format!("{name} must be greater than zero"),
            ));
        }
    }

    for (name, value) in [
        (
            "minimum_support.min_train_trades_per_fold",
            plan.minimum_support.min_train_trades_per_fold,
        ),
        (
            "minimum_support.min_validation_trades_per_fold",
            plan.minimum_support.min_validation_trades_per_fold,
        ),
        (
            "minimum_support.min_final_test_trades",
            plan.minimum_support.min_final_test_trades,
        ),
    ] {
        if value == 0 {
            return Err(temporal_error(
                VcetTemporalErrorCode::InvalidPlan,
                format!("{name} must be greater than zero"),
            ));
        }
    }

    if plan.train_bars < plan.minimum_support.min_train_bars {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidPlan,
            "train_bars is below minimum_support.min_train_bars",
        ));
    }
    if plan.validation_bars < plan.minimum_support.min_validation_bars {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidPlan,
            "validation_bars is below minimum_support.min_validation_bars",
        ));
    }
    if plan.final_test_bars < plan.minimum_support.min_final_test_bars {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidPlan,
            "final_test_bars is below minimum_support.min_final_test_bars",
        ));
    }

    let validation_plus_embargo = checked_add(
        plan.validation_bars,
        plan.embargo_bars,
        "validation_bars + embargo_bars",
    )?;
    if plan.step_bars < validation_plus_embargo {
        return Err(temporal_error(
            VcetTemporalErrorCode::IntervalOverlap,
            format!(
                "step_bars ({}) must be >= validation_bars + embargo_bars ({validation_plus_embargo}) so validation folds and embargo intervals do not overlap",
                plan.step_bars
            ),
        ));
    }

    minimum_required_vcet_temporal_bars(plan)?;
    Ok(())
}

/// Minimum selected bar count needed to construct the plan's required fold count and final test.
pub fn minimum_required_vcet_temporal_bars(
    plan: &VcetTemporalValidationPlanV1,
) -> VcetTemporalResult<usize> {
    let fold_offset = checked_mul(
        plan.minimum_support.min_folds.saturating_sub(1),
        plan.step_bars,
        "minimum fold offset",
    )?;
    let mut required = plan.warmup_bars;
    for (value, field) in [
        (plan.train_bars, "train bars"),
        (plan.purge_bars, "validation purge bars"),
        (plan.validation_bars, "validation bars"),
        (fold_offset, "minimum fold offset"),
        (plan.embargo_bars, "last validation embargo bars"),
        (plan.purge_bars, "final-test purge bars"),
        (plan.final_test_bars, "final-test bars"),
    ] {
        required = checked_add(required, value, field)?;
    }
    Ok(required)
}

pub fn vcet_temporal_plan_hash_input(
    plan: &VcetTemporalValidationPlanV1,
) -> VcetTemporalResult<VcetTemporalPlanHashInputV1> {
    validate_vcet_temporal_plan(plan)?;
    Ok(VcetTemporalPlanHashInputV1 {
        hash_contract: VCET_TEMPORAL_VALIDATION_PLAN_HASH_CONTRACT.to_string(),
        schema_version: VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
        plan: plan.clone(),
    })
}

pub fn vcet_temporal_plan_identity(
    plan: &VcetTemporalValidationPlanV1,
) -> VcetTemporalResult<VcetTemporalPlanIdentityV1> {
    let hash_input = vcet_temporal_plan_hash_input(plan)?;
    let bytes = serde_json::to_vec(&hash_input).map_err(|error| {
        temporal_error(
            VcetTemporalErrorCode::InvalidPlan,
            format!("failed to serialize VCET temporal plan hash input: {error}"),
        )
    })?;
    let digest = Sha256::digest(bytes);
    let mut plan_sha256 = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut plan_sha256, "{byte:02x}");
    }
    let plan_id = format!(
        "vcet-temporal-v{}-{}",
        plan.schema_version,
        &plan_sha256[..24]
    );
    Ok(VcetTemporalPlanIdentityV1 {
        schema_version: plan.schema_version,
        plan_id,
        plan_sha256,
    })
}

fn validate_timestamp_series(timestamps_utc_ms: &[i64]) -> VcetTemporalResult<()> {
    if timestamps_utc_ms.is_empty() {
        return Err(temporal_error(
            VcetTemporalErrorCode::EmptyDataset,
            "VCET temporal dataset must contain at least one candle timestamp",
        ));
    }
    if timestamps_utc_ms.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(temporal_error(
            VcetTemporalErrorCode::NonMonotonicTimestamps,
            "VCET candle timestamps must be strictly increasing with no duplicates",
        ));
    }
    Ok(())
}

fn validate_session_series(
    timestamps_utc_ms: &[i64],
    session: VcetTemporalSessionSeriesRef<'_>,
) -> VcetTemporalResult<()> {
    if session.calendar_id.trim().is_empty() {
        return Err(temporal_error(
            VcetTemporalErrorCode::MissingSessionSeries,
            "VCET session calendar_id must not be empty",
        ));
    }
    if session.session_open_utc_ms.len() != timestamps_utc_ms.len() {
        return Err(temporal_error(
            VcetTemporalErrorCode::DatasetShapeMismatch,
            format!(
                "VCET session series length {} does not match timestamp length {}",
                session.session_open_utc_ms.len(),
                timestamps_utc_ms.len()
            ),
        ));
    }
    if session
        .session_open_utc_ms
        .windows(2)
        .any(|pair| pair[0] > pair[1])
    {
        return Err(temporal_error(
            VcetTemporalErrorCode::NonMonotonicTimestamps,
            "VCET canonical session timestamps must be nondecreasing",
        ));
    }
    Ok(())
}

pub fn observe_vcet_temporal_dataset(
    dataset_id: impl Into<String>,
    timestamps_utc_ms: &[i64],
    session: Option<VcetTemporalSessionSeriesRef<'_>>,
) -> VcetTemporalResult<VcetTemporalDatasetObservationV1> {
    let dataset_id = dataset_id.into();
    if dataset_id.trim().is_empty() {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidPlan,
            "VCET temporal dataset_id must not be empty",
        ));
    }
    validate_timestamp_series(timestamps_utc_ms)?;

    let (observed_session, observed_session_count) = if let Some(session) = session {
        validate_session_series(timestamps_utc_ms, session)?;
        let session_count = 1 + session
            .session_open_utc_ms
            .windows(2)
            .filter(|pair| pair[0] != pair[1])
            .count();
        (
            Some(VcetTemporalSessionBoundsV1 {
                calendar_id: session.calendar_id.to_string(),
                first_session_open_utc_ms_inclusive: session.session_open_utc_ms[0],
                last_session_open_utc_ms_inclusive: session.session_open_utc_ms
                    [session.session_open_utc_ms.len() - 1],
            }),
            Some(session_count),
        )
    } else {
        (None, None)
    };

    Ok(VcetTemporalDatasetObservationV1 {
        dataset_id,
        bar_count: timestamps_utc_ms.len(),
        observed_utc: VcetTemporalUtcBoundsV1 {
            start_utc_ms_inclusive: timestamps_utc_ms[0],
            end_utc_ms_inclusive: timestamps_utc_ms[timestamps_utc_ms.len() - 1],
        },
        observed_session,
        observed_session_count,
    })
}

pub fn validate_vcet_temporal_observation(
    plan: &VcetTemporalValidationPlanV1,
    observation: &VcetTemporalDatasetObservationV1,
) -> VcetTemporalResult<()> {
    validate_vcet_temporal_plan(plan)?;
    if observation.dataset_id.trim().is_empty() || observation.bar_count == 0 {
        return Err(temporal_error(
            VcetTemporalErrorCode::EmptyDataset,
            "VCET temporal dataset observation is empty or has no dataset_id",
        ));
    }
    validate_utc_bounds(observation.observed_utc, "observation.observed_utc")?;
    match (
        observation.observed_session.as_ref(),
        observation.observed_session_count,
    ) {
        (Some(observed_session), Some(session_count)) => {
            validate_session_bounds(observed_session, "observation.observed_session")?;
            if session_count == 0 || session_count > observation.bar_count {
                return Err(temporal_error(
                    VcetTemporalErrorCode::DatasetShapeMismatch,
                    format!(
                        "dataset '{}' observed session count {session_count} is invalid for {} bars",
                        observation.dataset_id, observation.bar_count
                    ),
                ));
            }
        }
        (Some(_), None) | (None, Some(_)) => {
            return Err(temporal_error(
                VcetTemporalErrorCode::DatasetShapeMismatch,
                format!(
                    "dataset '{}' must provide observed_session and observed_session_count together",
                    observation.dataset_id
                ),
            ));
        }
        (None, None) => {}
    }
    let requested_utc = plan.requested_bounds.utc;
    if observation.observed_utc.start_utc_ms_inclusive > requested_utc.start_utc_ms_inclusive
        || observation.observed_utc.end_utc_ms_inclusive < requested_utc.end_utc_ms_inclusive
    {
        return Err(temporal_error(
            VcetTemporalErrorCode::CoverageInadequate,
            format!(
                "dataset '{}' observed UTC bounds [{}, {}] do not cover requested bounds [{}, {}]",
                observation.dataset_id,
                observation.observed_utc.start_utc_ms_inclusive,
                observation.observed_utc.end_utc_ms_inclusive,
                requested_utc.start_utc_ms_inclusive,
                requested_utc.end_utc_ms_inclusive
            ),
        ));
    }

    match (
        plan.requested_bounds.session.as_ref(),
        observation.observed_session.as_ref(),
    ) {
        (Some(_), None) => {
            return Err(temporal_error(
                VcetTemporalErrorCode::MissingSessionSeries,
                format!(
                    "dataset '{}' has no session observation but the temporal plan requires one",
                    observation.dataset_id
                ),
            ));
        }
        (Some(requested), Some(observed)) => {
            validate_session_bounds(observed, "observation.observed_session")?;
            if observed.calendar_id != requested.calendar_id {
                return Err(temporal_error(
                    VcetTemporalErrorCode::SessionCalendarMismatch,
                    format!(
                        "dataset '{}' session calendar '{}' does not match requested calendar '{}'",
                        observation.dataset_id, observed.calendar_id, requested.calendar_id
                    ),
                ));
            }
            if observed.first_session_open_utc_ms_inclusive
                > requested.first_session_open_utc_ms_inclusive
                || observed.last_session_open_utc_ms_inclusive
                    < requested.last_session_open_utc_ms_inclusive
            {
                return Err(temporal_error(
                    VcetTemporalErrorCode::CoverageInadequate,
                    format!(
                        "dataset '{}' observed sessions [{}, {}] do not cover requested sessions [{}, {}]",
                        observation.dataset_id,
                        observed.first_session_open_utc_ms_inclusive,
                        observed.last_session_open_utc_ms_inclusive,
                        requested.first_session_open_utc_ms_inclusive,
                        requested.last_session_open_utc_ms_inclusive
                    ),
                ));
            }
        }
        (None, _) => {}
    }
    Ok(())
}

fn range_utc_bounds(
    timestamps_utc_ms: &[i64],
    range: VcetTemporalBarRangeV1,
) -> VcetTemporalResult<VcetTemporalUtcBoundsV1> {
    if range.is_empty() || range.end_index_exclusive > timestamps_utc_ms.len() {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            format!(
                "bar range [{}, {}) is empty or outside timestamp length {}",
                range.start_index,
                range.end_index_exclusive,
                timestamps_utc_ms.len()
            ),
        ));
    }
    Ok(VcetTemporalUtcBoundsV1 {
        start_utc_ms_inclusive: timestamps_utc_ms[range.start_index],
        end_utc_ms_inclusive: timestamps_utc_ms[range.end_index_exclusive - 1],
    })
}

fn range_session_bounds(
    session: Option<VcetTemporalSessionSeriesRef<'_>>,
    range: VcetTemporalBarRangeV1,
) -> VcetTemporalResult<Option<VcetTemporalSessionBoundsV1>> {
    let Some(session) = session else {
        return Ok(None);
    };
    if range.is_empty() || range.end_index_exclusive > session.session_open_utc_ms.len() {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            "bar range is empty or outside session series",
        ));
    }
    Ok(Some(VcetTemporalSessionBoundsV1 {
        calendar_id: session.calendar_id.to_string(),
        first_session_open_utc_ms_inclusive: session.session_open_utc_ms[range.start_index],
        last_session_open_utc_ms_inclusive: session.session_open_utc_ms
            [range.end_index_exclusive - 1],
    }))
}

pub fn select_vcet_temporal_dataset(
    plan: &VcetTemporalValidationPlanV1,
    dataset_id: impl Into<String>,
    timestamps_utc_ms: &[i64],
    session: Option<VcetTemporalSessionSeriesRef<'_>>,
) -> VcetTemporalResult<VcetTemporalDatasetSliceV1> {
    validate_vcet_temporal_plan(plan)?;
    let observation = observe_vcet_temporal_dataset(dataset_id, timestamps_utc_ms, session)?;
    validate_vcet_temporal_observation(plan, &observation)?;

    let requested_utc = plan.requested_bounds.utc;
    let utc_start = timestamps_utc_ms
        .partition_point(|timestamp| *timestamp < requested_utc.start_utc_ms_inclusive);
    let utc_end = timestamps_utc_ms
        .partition_point(|timestamp| *timestamp <= requested_utc.end_utc_ms_inclusive);
    let mut start_index = utc_start;
    let mut end_index_exclusive = utc_end;

    if let Some(requested_session) = plan.requested_bounds.session.as_ref() {
        let session = session.ok_or_else(|| {
            temporal_error(
                VcetTemporalErrorCode::MissingSessionSeries,
                "VCET temporal plan requests session bounds but no session series was provided",
            )
        })?;
        if session.calendar_id != requested_session.calendar_id {
            return Err(temporal_error(
                VcetTemporalErrorCode::SessionCalendarMismatch,
                format!(
                    "provided session calendar '{}' does not match plan calendar '{}'",
                    session.calendar_id, requested_session.calendar_id
                ),
            ));
        }
        let session_start = session.session_open_utc_ms.partition_point(|timestamp| {
            *timestamp < requested_session.first_session_open_utc_ms_inclusive
        });
        let session_end = session.session_open_utc_ms.partition_point(|timestamp| {
            *timestamp <= requested_session.last_session_open_utc_ms_inclusive
        });
        start_index = start_index.max(session_start);
        end_index_exclusive = end_index_exclusive.min(session_end);
    }

    let selected_bars = VcetTemporalBarRangeV1 {
        start_index,
        end_index_exclusive,
    };
    if selected_bars.is_empty() {
        return Err(temporal_error(
            VcetTemporalErrorCode::CoverageInadequate,
            format!(
                "dataset '{}' has no candles in the requested UTC/session intersection",
                observation.dataset_id
            ),
        ));
    }

    Ok(VcetTemporalDatasetSliceV1 {
        observation,
        requested_bounds: plan.requested_bounds.clone(),
        selected_bars,
        selected_utc: range_utc_bounds(timestamps_utc_ms, selected_bars)?,
        selected_session: range_session_bounds(session, selected_bars)?,
    })
}

fn evaluation_slice(
    timestamps_utc_ms: &[i64],
    session: Option<VcetTemporalSessionSeriesRef<'_>>,
    scored_bars: VcetTemporalBarRangeV1,
    warmup_bars: usize,
) -> VcetTemporalResult<VcetTemporalEvaluationSliceV1> {
    let warmup_start = scored_bars.start_index.checked_sub(warmup_bars).ok_or_else(|| {
        temporal_error(
            VcetTemporalErrorCode::InsufficientBars,
            format!(
                "scored interval beginning at {} does not have {warmup_bars} preceding warm-up bars",
                scored_bars.start_index
            ),
        )
    })?;
    Ok(VcetTemporalEvaluationSliceV1 {
        warmup_bars: VcetTemporalBarRangeV1 {
            start_index: warmup_start,
            end_index_exclusive: scored_bars.start_index,
        },
        scored_bars,
        scored_utc: range_utc_bounds(timestamps_utc_ms, scored_bars)?,
        scored_session: range_session_bounds(session, scored_bars)?,
    })
}

pub fn plan_vcet_temporal_folds(
    plan: &VcetTemporalValidationPlanV1,
    dataset_id: impl Into<String>,
    timestamps_utc_ms: &[i64],
    session: Option<VcetTemporalSessionSeriesRef<'_>>,
) -> VcetTemporalResult<VcetTemporalLayoutV1> {
    validate_vcet_temporal_plan(plan)?;
    let dataset = select_vcet_temporal_dataset(plan, dataset_id, timestamps_utc_ms, session)?;
    let selected = dataset.selected_bars;
    let required = minimum_required_vcet_temporal_bars(plan)?;
    if selected.len() < required {
        return Err(temporal_error(
            VcetTemporalErrorCode::InsufficientBars,
            format!(
                "dataset '{}' has {} selected bars but the VCET temporal plan requires at least {required}",
                dataset.observation.dataset_id,
                selected.len()
            ),
        ));
    }

    let final_test_start = selected
        .end_index_exclusive
        .checked_sub(plan.final_test_bars)
        .ok_or_else(|| {
            temporal_error(
                VcetTemporalErrorCode::InsufficientBars,
                "selected dataset is shorter than final_test_bars",
            )
        })?;
    let final_purge_start = final_test_start
        .checked_sub(plan.purge_bars)
        .ok_or_else(|| {
            temporal_error(
                VcetTemporalErrorCode::InsufficientBars,
                "selected dataset has no room for final-test purge",
            )
        })?;
    let first_train_start =
        checked_add(selected.start_index, plan.warmup_bars, "first train start")?;
    let first_train_end = checked_add(first_train_start, plan.train_bars, "first train end")?;

    let mut folds = Vec::new();
    let mut fold_index = 0usize;
    loop {
        let offset = checked_mul(fold_index, plan.step_bars, "fold offset")?;
        let train_end = checked_add(first_train_end, offset, "fold train end")?;
        let validation_start = checked_add(train_end, plan.purge_bars, "validation start")?;
        let validation_end = checked_add(validation_start, plan.validation_bars, "validation end")?;
        let embargo_end = checked_add(validation_end, plan.embargo_bars, "embargo end")?;
        if embargo_end > final_purge_start {
            break;
        }

        let train_start = match plan.fold_mode {
            VcetTemporalFoldModeV1::Anchored => first_train_start,
            VcetTemporalFoldModeV1::Rolling => {
                train_end.checked_sub(plan.train_bars).ok_or_else(|| {
                    temporal_error(
                        VcetTemporalErrorCode::ArithmeticOverflow,
                        "rolling train start underflow",
                    )
                })?
            }
        };
        let train_range = VcetTemporalBarRangeV1 {
            start_index: train_start,
            end_index_exclusive: train_end,
        };
        let validation_range = VcetTemporalBarRangeV1 {
            start_index: validation_start,
            end_index_exclusive: validation_end,
        };
        folds.push(VcetTemporalFoldV1 {
            fold_index,
            train: evaluation_slice(timestamps_utc_ms, session, train_range, plan.warmup_bars)?,
            purge_before_validation: VcetTemporalBarRangeV1 {
                start_index: train_end,
                end_index_exclusive: validation_start,
            },
            validation: evaluation_slice(
                timestamps_utc_ms,
                session,
                validation_range,
                plan.warmup_bars,
            )?,
            embargo_after_validation: VcetTemporalBarRangeV1 {
                start_index: validation_end,
                end_index_exclusive: embargo_end,
            },
        });
        fold_index = checked_add(fold_index, 1, "fold index")?;
    }

    if folds.len() < plan.minimum_support.min_folds {
        return Err(temporal_error(
            VcetTemporalErrorCode::InsufficientFolds,
            format!(
                "dataset '{}' produced {} folds but the plan requires at least {}",
                dataset.observation.dataset_id,
                folds.len(),
                plan.minimum_support.min_folds
            ),
        ));
    }

    let final_test_range = VcetTemporalBarRangeV1 {
        start_index: final_test_start,
        end_index_exclusive: selected.end_index_exclusive,
    };
    let last_embargo_end = folds
        .last()
        .map(|fold| fold.embargo_after_validation.end_index_exclusive)
        .unwrap_or(first_train_start);
    let layout = VcetTemporalLayoutV1 {
        identity: vcet_temporal_plan_identity(plan)?,
        dataset,
        folds,
        sealed_selection_remainder: VcetTemporalBarRangeV1 {
            start_index: last_embargo_end,
            end_index_exclusive: final_purge_start,
        },
        purge_before_final_test: VcetTemporalBarRangeV1 {
            start_index: final_purge_start,
            end_index_exclusive: final_test_start,
        },
        final_test: evaluation_slice(
            timestamps_utc_ms,
            session,
            final_test_range,
            plan.warmup_bars,
        )?,
        position_boundary_policy: plan.position_boundary_policy,
    };
    validate_vcet_temporal_layout(plan, &layout)?;
    Ok(layout)
}

fn require_range_within(
    name: &str,
    range: VcetTemporalBarRangeV1,
    outer: VcetTemporalBarRangeV1,
    allow_empty: bool,
) -> VcetTemporalResult<()> {
    if (!allow_empty && range.is_empty())
        || range.start_index < outer.start_index
        || range.end_index_exclusive > outer.end_index_exclusive
        || range.start_index > range.end_index_exclusive
    {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            format!(
                "{name} range [{}, {}) is invalid for selected range [{}, {})",
                range.start_index,
                range.end_index_exclusive,
                outer.start_index,
                outer.end_index_exclusive
            ),
        ));
    }
    Ok(())
}

fn validate_evaluation_slice(
    name: &str,
    slice: &VcetTemporalEvaluationSliceV1,
    outer: VcetTemporalBarRangeV1,
    expected_scored_len: usize,
    expected_warmup_len: usize,
) -> VcetTemporalResult<()> {
    require_range_within(
        &format!("{name}.warmup_bars"),
        slice.warmup_bars,
        outer,
        expected_warmup_len == 0,
    )?;
    require_range_within(
        &format!("{name}.scored_bars"),
        slice.scored_bars,
        outer,
        false,
    )?;
    if slice.warmup_bars.len() != expected_warmup_len
        || slice.scored_bars.len() != expected_scored_len
        || slice.warmup_bars.end_index_exclusive != slice.scored_bars.start_index
    {
        return Err(temporal_error(
            VcetTemporalErrorCode::InvalidBounds,
            format!("{name} warm-up/scored range lengths or adjacency do not match the plan"),
        ));
    }
    validate_utc_bounds(slice.scored_utc, &format!("{name}.scored_utc"))?;
    if let Some(bounds) = slice.scored_session.as_ref() {
        validate_session_bounds(bounds, &format!("{name}.scored_session"))?;
    }
    Ok(())
}

pub fn validate_vcet_temporal_layout(
    plan: &VcetTemporalValidationPlanV1,
    layout: &VcetTemporalLayoutV1,
) -> VcetTemporalResult<()> {
    validate_vcet_temporal_plan(plan)?;
    let expected_identity = vcet_temporal_plan_identity(plan)?;
    if layout.identity != expected_identity {
        return Err(temporal_error(
            VcetTemporalErrorCode::IdentityMismatch,
            "VCET temporal layout plan identity does not match the supplied plan",
        ));
    }
    if layout.position_boundary_policy != plan.position_boundary_policy {
        return Err(temporal_error(
            VcetTemporalErrorCode::IdentityMismatch,
            "VCET temporal layout position-boundary policy does not match the supplied plan",
        ));
    }
    validate_vcet_temporal_observation(plan, &layout.dataset.observation)?;
    if layout.dataset.requested_bounds != plan.requested_bounds {
        return Err(temporal_error(
            VcetTemporalErrorCode::IdentityMismatch,
            "VCET temporal dataset slice requested bounds do not match the supplied plan",
        ));
    }
    let selected = layout.dataset.selected_bars;
    require_range_within(
        "dataset.selected_bars",
        selected,
        VcetTemporalBarRangeV1 {
            start_index: 0,
            end_index_exclusive: layout.dataset.observation.bar_count,
        },
        false,
    )?;
    validate_utc_bounds(layout.dataset.selected_utc, "dataset.selected_utc")?;
    if layout.dataset.selected_utc.start_utc_ms_inclusive
        < plan.requested_bounds.utc.start_utc_ms_inclusive
        || layout.dataset.selected_utc.end_utc_ms_inclusive
            > plan.requested_bounds.utc.end_utc_ms_inclusive
    {
        return Err(temporal_error(
            VcetTemporalErrorCode::CoverageInadequate,
            "VCET temporal selected UTC bounds escape the requested plan bounds",
        ));
    }
    match (
        plan.requested_bounds.session.as_ref(),
        layout.dataset.selected_session.as_ref(),
    ) {
        (Some(_), None) => {
            return Err(temporal_error(
                VcetTemporalErrorCode::MissingSessionSeries,
                "VCET temporal dataset slice is missing required selected session bounds",
            ));
        }
        (Some(requested), Some(selected_session)) => {
            validate_session_bounds(selected_session, "dataset.selected_session")?;
            if selected_session.calendar_id != requested.calendar_id {
                return Err(temporal_error(
                    VcetTemporalErrorCode::SessionCalendarMismatch,
                    "VCET temporal selected session calendar does not match the plan",
                ));
            }
            if selected_session.first_session_open_utc_ms_inclusive
                < requested.first_session_open_utc_ms_inclusive
                || selected_session.last_session_open_utc_ms_inclusive
                    > requested.last_session_open_utc_ms_inclusive
            {
                return Err(temporal_error(
                    VcetTemporalErrorCode::CoverageInadequate,
                    "VCET temporal selected session bounds escape the requested plan bounds",
                ));
            }
        }
        (None, _) => {}
    }
    if layout.folds.len() < plan.minimum_support.min_folds {
        return Err(temporal_error(
            VcetTemporalErrorCode::InsufficientFolds,
            format!(
                "VCET temporal layout contains {} folds; expected at least {}",
                layout.folds.len(),
                plan.minimum_support.min_folds
            ),
        ));
    }

    let first_train_start = checked_add(
        selected.start_index,
        plan.warmup_bars,
        "layout first train start",
    )?;
    let mut prior_validation_start = None;
    let mut prior_embargo_end = None;
    for (expected_index, fold) in layout.folds.iter().enumerate() {
        if fold.fold_index != expected_index {
            return Err(temporal_error(
                VcetTemporalErrorCode::InvalidPlan,
                format!(
                    "VCET temporal fold index {} does not match expected index {expected_index}",
                    fold.fold_index
                ),
            ));
        }
        validate_evaluation_slice(
            &format!("fold[{expected_index}].train"),
            &fold.train,
            selected,
            match plan.fold_mode {
                VcetTemporalFoldModeV1::Anchored => {
                    let offset = checked_mul(
                        expected_index,
                        plan.step_bars,
                        "anchored train length offset",
                    )?;
                    checked_add(plan.train_bars, offset, "anchored train length")?
                }
                VcetTemporalFoldModeV1::Rolling => plan.train_bars,
            },
            plan.warmup_bars,
        )?;
        validate_evaluation_slice(
            &format!("fold[{expected_index}].validation"),
            &fold.validation,
            selected,
            plan.validation_bars,
            plan.warmup_bars,
        )?;
        require_range_within(
            &format!("fold[{expected_index}].purge_before_validation"),
            fold.purge_before_validation,
            selected,
            plan.purge_bars == 0,
        )?;
        require_range_within(
            &format!("fold[{expected_index}].embargo_after_validation"),
            fold.embargo_after_validation,
            selected,
            plan.embargo_bars == 0,
        )?;

        if fold.train.scored_bars.start_index
            != match plan.fold_mode {
                VcetTemporalFoldModeV1::Anchored => first_train_start,
                VcetTemporalFoldModeV1::Rolling => {
                    let offset =
                        checked_mul(expected_index, plan.step_bars, "rolling train offset")?;
                    checked_add(first_train_start, offset, "rolling train start")?
                }
            }
            || fold.train.scored_bars.end_index_exclusive
                != fold.purge_before_validation.start_index
            || fold.purge_before_validation.len() != plan.purge_bars
            || fold.purge_before_validation.end_index_exclusive
                != fold.validation.scored_bars.start_index
            || fold.validation.scored_bars.end_index_exclusive
                != fold.embargo_after_validation.start_index
            || fold.embargo_after_validation.len() != plan.embargo_bars
        {
            return Err(temporal_error(
                VcetTemporalErrorCode::IntervalOverlap,
                format!(
                    "VCET temporal fold {expected_index} phase boundaries do not match the plan"
                ),
            ));
        }

        if let Some(previous_start) = prior_validation_start {
            let expected_start = checked_add(previous_start, plan.step_bars, "validation step")?;
            if fold.validation.scored_bars.start_index != expected_start {
                return Err(temporal_error(
                    VcetTemporalErrorCode::IntervalOverlap,
                    format!("VCET temporal fold {expected_index} does not honor step_bars"),
                ));
            }
        }
        if let Some(previous_end) = prior_embargo_end {
            if fold.validation.scored_bars.start_index < previous_end {
                return Err(temporal_error(
                    VcetTemporalErrorCode::IntervalOverlap,
                    format!("VCET temporal fold {expected_index} overlaps a prior embargo"),
                ));
            }
        }
        prior_validation_start = Some(fold.validation.scored_bars.start_index);
        prior_embargo_end = Some(fold.embargo_after_validation.end_index_exclusive);
    }

    validate_evaluation_slice(
        "final_test",
        &layout.final_test,
        selected,
        plan.final_test_bars,
        plan.warmup_bars,
    )?;
    require_range_within(
        "sealed_selection_remainder",
        layout.sealed_selection_remainder,
        selected,
        true,
    )?;
    require_range_within(
        "purge_before_final_test",
        layout.purge_before_final_test,
        selected,
        plan.purge_bars == 0,
    )?;
    let last_embargo_end = layout
        .folds
        .last()
        .map(|fold| fold.embargo_after_validation.end_index_exclusive)
        .unwrap_or(first_train_start);
    if layout.sealed_selection_remainder.start_index != last_embargo_end
        || layout.sealed_selection_remainder.end_index_exclusive
            != layout.purge_before_final_test.start_index
        || layout.purge_before_final_test.len() != plan.purge_bars
        || layout.purge_before_final_test.end_index_exclusive
            != layout.final_test.scored_bars.start_index
        || layout.final_test.scored_bars.end_index_exclusive != selected.end_index_exclusive
    {
        return Err(temporal_error(
            VcetTemporalErrorCode::IntervalOverlap,
            "VCET temporal final-test boundary, purge, or selection remainder is invalid",
        ));
    }
    Ok(())
}

pub fn validate_vcet_temporal_trade_support(
    plan: &VcetTemporalValidationPlanV1,
    layout: &VcetTemporalLayoutV1,
    support: &VcetTemporalTradeSupportV1,
) -> VcetTemporalResult<()> {
    validate_vcet_temporal_layout(plan, layout)?;
    if support.folds.len() != layout.folds.len() {
        return Err(temporal_error(
            VcetTemporalErrorCode::TradeSupportInadequate,
            format!(
                "VCET temporal trade evidence covers {} folds but the layout contains {}",
                support.folds.len(),
                layout.folds.len()
            ),
        ));
    }
    for (expected, observed) in layout.folds.iter().zip(&support.folds) {
        if observed.fold_index != expected.fold_index {
            return Err(temporal_error(
                VcetTemporalErrorCode::TradeSupportInadequate,
                format!(
                    "VCET temporal trade evidence fold {} does not match expected fold {}",
                    observed.fold_index, expected.fold_index
                ),
            ));
        }
        if observed.train_trades < plan.minimum_support.min_train_trades_per_fold
            || observed.validation_trades < plan.minimum_support.min_validation_trades_per_fold
        {
            return Err(temporal_error(
                VcetTemporalErrorCode::TradeSupportInadequate,
                format!(
                    "VCET temporal fold {} has train/validation trades {}/{} below required {}/{}",
                    observed.fold_index,
                    observed.train_trades,
                    observed.validation_trades,
                    plan.minimum_support.min_train_trades_per_fold,
                    plan.minimum_support.min_validation_trades_per_fold
                ),
            ));
        }
    }
    if support.final_test_trades < plan.minimum_support.min_final_test_trades {
        return Err(temporal_error(
            VcetTemporalErrorCode::TradeSupportInadequate,
            format!(
                "VCET temporal final test has {} trades; at least {} are required",
                support.final_test_trades, plan.minimum_support.min_final_test_trades
            ),
        ));
    }
    Ok(())
}

/// Require every dataset to use exactly the same scored UTC/session intervals.
///
/// This is the fail-closed cross-sectional companion to per-dataset planning: missing candles may
/// shift a bar-count-based fold even when every dataset covers the outer requested bounds. A
/// campaign should call this before treating folds as common-time cross-sectional evidence.
pub fn validate_vcet_temporal_cross_dataset_alignment(
    plan: &VcetTemporalValidationPlanV1,
    layouts: &[VcetTemporalLayoutV1],
) -> VcetTemporalResult<()> {
    let Some(reference) = layouts.first() else {
        return Err(temporal_error(
            VcetTemporalErrorCode::EmptyDataset,
            "VCET temporal cross-dataset alignment requires at least one layout",
        ));
    };
    validate_vcet_temporal_layout(plan, reference)?;

    for (index, layout) in layouts.iter().enumerate().skip(1) {
        validate_vcet_temporal_layout(plan, layout)?;
        if layouts[..index].iter().any(|prior| {
            prior.dataset.observation.dataset_id == layout.dataset.observation.dataset_id
        }) {
            return Err(temporal_error(
                VcetTemporalErrorCode::CrossDatasetMisalignment,
                format!(
                    "duplicate VCET temporal dataset_id '{}' in aligned layout set",
                    layout.dataset.observation.dataset_id
                ),
            ));
        }
        if layout.folds.len() != reference.folds.len() {
            return Err(temporal_error(
                VcetTemporalErrorCode::CrossDatasetMisalignment,
                format!(
                    "dataset '{}' has {} temporal folds; reference dataset '{}' has {}",
                    layout.dataset.observation.dataset_id,
                    layout.folds.len(),
                    reference.dataset.observation.dataset_id,
                    reference.folds.len()
                ),
            ));
        }
        for (reference_fold, observed_fold) in reference.folds.iter().zip(&layout.folds) {
            if observed_fold.train.scored_utc != reference_fold.train.scored_utc
                || observed_fold.train.scored_session != reference_fold.train.scored_session
                || observed_fold.validation.scored_utc != reference_fold.validation.scored_utc
                || observed_fold.validation.scored_session
                    != reference_fold.validation.scored_session
            {
                return Err(temporal_error(
                    VcetTemporalErrorCode::CrossDatasetMisalignment,
                    format!(
                        "dataset '{}' fold {} UTC/session intervals do not match reference dataset '{}'",
                        layout.dataset.observation.dataset_id,
                        observed_fold.fold_index,
                        reference.dataset.observation.dataset_id
                    ),
                ));
            }
        }
        if layout.final_test.scored_utc != reference.final_test.scored_utc
            || layout.final_test.scored_session != reference.final_test.scored_session
        {
            return Err(temporal_error(
                VcetTemporalErrorCode::CrossDatasetMisalignment,
                format!(
                    "dataset '{}' final-test UTC/session interval does not match reference dataset '{}'",
                    layout.dataset.observation.dataset_id,
                    reference.dataset.observation.dataset_id
                ),
            ));
        }
    }
    Ok(())
}

pub fn slice_vcet_temporal_range<T>(
    values: &[T],
    range: VcetTemporalBarRangeV1,
) -> VcetTemporalResult<&[T]> {
    values
        .get(range.start_index..range.end_index_exclusive)
        .ok_or_else(|| {
            temporal_error(
                VcetTemporalErrorCode::DatasetShapeMismatch,
                format!(
                    "VCET temporal bar range [{}, {}) is outside value length {}",
                    range.start_index,
                    range.end_index_exclusive,
                    values.len()
                ),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION_1: i64 = 1_700_000_000_000;
    const SESSION_2: i64 = 1_700_086_400_000;

    fn test_plan(mode: VcetTemporalFoldModeV1) -> VcetTemporalValidationPlanV1 {
        VcetTemporalValidationPlanV1 {
            schema_version: VCET_TEMPORAL_VALIDATION_PLAN_SCHEMA_VERSION,
            requested_bounds: VcetTemporalRequestedBoundsV1 {
                utc: VcetTemporalUtcBoundsV1 {
                    start_utc_ms_inclusive: 1_000,
                    end_utc_ms_inclusive: 29_000,
                },
                session: None,
            },
            fold_mode: mode,
            train_bars: 5,
            validation_bars: 3,
            final_test_bars: 4,
            step_bars: 4,
            purge_bars: 1,
            embargo_bars: 1,
            warmup_bars: 2,
            position_boundary_policy: VcetTemporalPositionBoundaryPolicyV1::ResetFlatForceLiquidate,
            minimum_support: VcetTemporalMinimumSupportV1 {
                min_folds: 2,
                min_train_bars: 5,
                min_validation_bars: 3,
                min_final_test_bars: 4,
                min_train_trades_per_fold: 2,
                min_validation_trades_per_fold: 1,
                min_final_test_trades: 1,
            },
        }
    }

    fn timestamps(count: usize) -> Vec<i64> {
        (0..count).map(|index| index as i64 * 1_000).collect()
    }

    #[test]
    fn plan_identity_is_deterministic_and_semantic() {
        let plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        let first = vcet_temporal_plan_identity(&plan).expect("first identity");
        let second = vcet_temporal_plan_identity(&plan).expect("second identity");
        assert_eq!(first, second);
        assert_eq!(first.plan_sha256.len(), 64);
        assert!(first.plan_id.starts_with("vcet-temporal-v1-"));

        let mut changed = plan;
        changed.embargo_bars += 1;
        changed.step_bars += 1;
        assert_ne!(
            first,
            vcet_temporal_plan_identity(&changed).expect("changed identity")
        );
    }

    #[test]
    fn plan_validation_fails_closed_on_schema_overlap_and_zero_trade_minimum() {
        let mut plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.schema_version = 2;
        assert_eq!(
            validate_vcet_temporal_plan(&plan).unwrap_err().code,
            VcetTemporalErrorCode::UnsupportedSchemaVersion
        );

        plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.step_bars = plan.validation_bars;
        assert_eq!(
            validate_vcet_temporal_plan(&plan).unwrap_err().code,
            VcetTemporalErrorCode::IntervalOverlap
        );

        plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.minimum_support.min_final_test_trades = 0;
        assert_eq!(
            validate_vcet_temporal_plan(&plan).unwrap_err().code,
            VcetTemporalErrorCode::InvalidPlan
        );
    }

    #[test]
    fn dataset_observation_rejects_duplicates_and_session_shape_mismatch() {
        assert_eq!(
            observe_vcet_temporal_dataset("dup", &[1, 2, 2], None)
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::NonMonotonicTimestamps
        );
        let session = VcetTemporalSessionSeriesRef {
            calendar_id: "XNYS",
            session_open_utc_ms: &[SESSION_1],
        };
        assert_eq!(
            observe_vcet_temporal_dataset("shape", &[1, 2], Some(session))
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::DatasetShapeMismatch
        );
    }

    #[test]
    fn selection_intersects_closed_utc_and_session_bounds() {
        let timestamps = timestamps(12);
        let sessions = [
            SESSION_1, SESSION_1, SESSION_1, SESSION_1, SESSION_1, SESSION_1, SESSION_2, SESSION_2,
            SESSION_2, SESSION_2, SESSION_2, SESSION_2,
        ];
        let mut plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.requested_bounds.utc = VcetTemporalUtcBoundsV1 {
            start_utc_ms_inclusive: 2_000,
            end_utc_ms_inclusive: 9_000,
        };
        plan.requested_bounds.session = Some(VcetTemporalSessionBoundsV1 {
            calendar_id: "XNYS".into(),
            first_session_open_utc_ms_inclusive: SESSION_2,
            last_session_open_utc_ms_inclusive: SESSION_2,
        });
        let selected = select_vcet_temporal_dataset(
            &plan,
            "sessions",
            &timestamps,
            Some(VcetTemporalSessionSeriesRef {
                calendar_id: "XNYS",
                session_open_utc_ms: &sessions,
            }),
        )
        .expect("selected dataset");
        assert_eq!(
            selected.selected_bars,
            VcetTemporalBarRangeV1 {
                start_index: 6,
                end_index_exclusive: 10
            }
        );
        assert_eq!(selected.observation.observed_session_count, Some(2));
    }

    #[test]
    fn requested_coverage_and_calendar_mismatch_fail_closed() {
        let timestamps = timestamps(30);
        let mut plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.requested_bounds.utc.start_utc_ms_inclusive = -1;
        assert_eq!(
            select_vcet_temporal_dataset(&plan, "short", &timestamps, None)
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::CoverageInadequate
        );

        plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.requested_bounds.session = Some(VcetTemporalSessionBoundsV1 {
            calendar_id: "XNYS".into(),
            first_session_open_utc_ms_inclusive: SESSION_1,
            last_session_open_utc_ms_inclusive: SESSION_2,
        });
        let sessions = vec![SESSION_1; 30];
        assert_eq!(
            select_vcet_temporal_dataset(
                &plan,
                "wrong-calendar",
                &timestamps,
                Some(VcetTemporalSessionSeriesRef {
                    calendar_id: "XNAS",
                    session_open_utc_ms: &sessions,
                }),
            )
            .unwrap_err()
            .code,
            VcetTemporalErrorCode::SessionCalendarMismatch
        );
    }

    #[test]
    fn anchored_layout_has_expanding_train_and_sealed_final_test() {
        let timestamps = timestamps(30);
        let plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        let layout = plan_vcet_temporal_folds(&plan, "anchored", &timestamps, None)
            .expect("anchored layout");

        assert_eq!(layout.folds.len(), 4);
        assert_eq!(layout.folds[0].train.scored_bars, range(3, 8));
        assert_eq!(layout.folds[0].purge_before_validation, range(8, 9));
        assert_eq!(layout.folds[0].validation.scored_bars, range(9, 12));
        assert_eq!(layout.folds[0].embargo_after_validation, range(12, 13));
        assert_eq!(layout.folds[1].train.scored_bars, range(3, 12));
        assert_eq!(layout.folds[2].train.scored_bars, range(3, 16));
        assert_eq!(layout.folds[3].train.scored_bars, range(3, 20));
        assert_eq!(layout.purge_before_final_test, range(25, 26));
        assert_eq!(layout.final_test.scored_bars, range(26, 30));
        assert!(
            layout
                .folds
                .last()
                .unwrap()
                .embargo_after_validation
                .end_index_exclusive
                <= layout.purge_before_final_test.start_index
        );
        validate_vcet_temporal_layout(&plan, &layout).expect("valid layout");
    }

    #[test]
    fn rolling_layout_keeps_fixed_train_length_and_moves_start() {
        let timestamps = timestamps(30);
        let plan = test_plan(VcetTemporalFoldModeV1::Rolling);
        let layout =
            plan_vcet_temporal_folds(&plan, "rolling", &timestamps, None).expect("rolling layout");
        assert_eq!(layout.folds.len(), 4);
        assert_eq!(layout.folds[0].train.scored_bars, range(3, 8));
        assert_eq!(layout.folds[1].train.scored_bars, range(7, 12));
        assert_eq!(layout.folds[2].train.scored_bars, range(11, 16));
        assert_eq!(layout.folds[3].train.scored_bars, range(15, 20));
        assert!(layout
            .folds
            .iter()
            .all(|fold| fold.train.scored_bars.len() == plan.train_bars));
    }

    #[test]
    fn insufficient_bars_fails_before_partial_layout_is_returned() {
        let timestamps = timestamps(20);
        let mut plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        plan.requested_bounds.utc.end_utc_ms_inclusive = 19_000;
        let error = plan_vcet_temporal_folds(&plan, "short", &timestamps, None).unwrap_err();
        assert_eq!(error.code, VcetTemporalErrorCode::InsufficientBars);
    }

    #[test]
    fn layout_identity_and_boundaries_are_revalidated() {
        let timestamps = timestamps(30);
        let plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        let mut layout =
            plan_vcet_temporal_folds(&plan, "tamper", &timestamps, None).expect("layout");
        layout.identity.plan_sha256.push('0');
        assert_eq!(
            validate_vcet_temporal_layout(&plan, &layout)
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::IdentityMismatch
        );

        layout = plan_vcet_temporal_folds(&plan, "tamper", &timestamps, None).expect("layout");
        layout.folds[0].validation.scored_bars.start_index -= 1;
        assert_eq!(
            validate_vcet_temporal_layout(&plan, &layout)
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::InvalidBounds
        );
    }

    #[test]
    fn trade_support_requires_every_fold_and_final_test() {
        let timestamps = timestamps(30);
        let plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        let layout = plan_vcet_temporal_folds(&plan, "trades", &timestamps, None).expect("layout");
        let mut support = VcetTemporalTradeSupportV1 {
            folds: layout
                .folds
                .iter()
                .map(|fold| VcetTemporalFoldTradeSupportV1 {
                    fold_index: fold.fold_index,
                    train_trades: 2,
                    validation_trades: 1,
                })
                .collect(),
            final_test_trades: 1,
        };
        validate_vcet_temporal_trade_support(&plan, &layout, &support).expect("adequate support");
        support.folds[1].validation_trades = 0;
        assert_eq!(
            validate_vcet_temporal_trade_support(&plan, &layout, &support)
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::TradeSupportInadequate
        );
    }

    #[test]
    fn cross_dataset_alignment_requires_identical_fold_timestamps() {
        let timestamps = timestamps(30);
        let plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        let first =
            plan_vcet_temporal_folds(&plan, "first", &timestamps, None).expect("first layout");
        let mut second =
            plan_vcet_temporal_folds(&plan, "second", &timestamps, None).expect("second layout");
        validate_vcet_temporal_cross_dataset_alignment(&plan, &[first.clone(), second.clone()])
            .expect("aligned layouts");

        second.folds[1].validation.scored_utc.end_utc_ms_inclusive -= 1;
        assert_eq!(
            validate_vcet_temporal_cross_dataset_alignment(&plan, &[first, second])
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::CrossDatasetMisalignment
        );
    }

    #[test]
    fn observation_requires_session_bounds_and_count_together() {
        let plan = test_plan(VcetTemporalFoldModeV1::Anchored);
        let mut observation =
            observe_vcet_temporal_dataset("dataset", &timestamps(30), None).expect("observation");
        observation.observed_session_count = Some(2);
        assert_eq!(
            validate_vcet_temporal_observation(&plan, &observation)
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::DatasetShapeMismatch
        );
    }

    #[test]
    fn range_slicing_is_checked() {
        let values = [10, 20, 30, 40];
        assert_eq!(
            slice_vcet_temporal_range(&values, range(1, 3)).expect("slice"),
            &[20, 30]
        );
        assert_eq!(
            slice_vcet_temporal_range(&values, range(3, 5))
                .unwrap_err()
                .code,
            VcetTemporalErrorCode::DatasetShapeMismatch
        );
    }

    fn range(start_index: usize, end_index_exclusive: usize) -> VcetTemporalBarRangeV1 {
        VcetTemporalBarRangeV1 {
            start_index,
            end_index_exclusive,
        }
    }
}
