use serde::{Deserialize, Serialize};
use std::fmt;

pub const VCET_DATASET_COVERAGE_SCHEMA_VERSION: u32 = 1;
pub const VCET_DATASET_COVERAGE_MAX_TIMESTAMPS: usize = 10_000_000;
pub const VCET_DATASET_COVERAGE_MAX_CADENCE_SAMPLE: usize = 65_536;
pub const VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS: i64 = 253_402_300_799_999;
pub const VCET_DATASET_COVERAGE_MAX_END_EXCLUSIVE_MS: i64 =
    VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS + 1;

pub const VCET_RAW_GAP_CAVEAT: &str = "Gap counts use raw adjacent timestamps and may classify expected market closures as gaps; provide a session interval mask for session-aware evidence.";
pub const VCET_SESSION_MASK_GAP_CAVEAT: &str = "Gap counts include only adjacent intervals enabled by the caller-provided session interval mask; the caller remains responsible for the mask's exchange-calendar correctness.";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VcetRequestedCoverageBoundsMs {
    pub start_ms: i64,
    pub end_exclusive_ms: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VcetObservedCoverageBoundsMs {
    pub min_ms: i64,
    pub max_ms: i64,
    pub span_ms: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VcetRequestedWindowCounts {
    pub before_start: usize,
    pub within: usize,
    pub at_or_after_end: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VcetTimestampOrderingEvidence {
    pub monotonic_non_decreasing: bool,
    pub strictly_increasing: bool,
    pub duplicate_interval_count: usize,
    pub descending_interval_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_duplicate_at_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_descending_at_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VcetGapAnalysisStatus {
    Complete,
    InsufficientObservations,
    InvalidTimestampOrder,
    NoEligibleIntervals,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VcetGapAnalysisBasis {
    RawAdjacentTimestamps,
    SessionMaskedAdjacentTimestamps,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VcetGapCadenceSource {
    SuppliedExpectedCadence,
    EstimatedUpperMedianPositiveDelta,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VcetGapEvidence {
    pub status: VcetGapAnalysisStatus,
    pub basis: VcetGapAnalysisBasis,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_cadence_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence_estimate_ms: Option<i64>,
    pub cadence_sample_interval_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence_used_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cadence_source: Option<VcetGapCadenceSource>,
    pub adjacent_interval_count: usize,
    pub analyzed_interval_count: usize,
    pub excluded_interval_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub irregular_interval_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_interval_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_interval_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longest_gap_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longest_gap_missing_interval_count: Option<u64>,
    pub caveat: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VcetDatasetCoverageEvidence {
    pub schema_version: u32,
    pub timestamp_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_bounds: Option<VcetRequestedCoverageBoundsMs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_bounds: Option<VcetObservedCoverageBoundsMs>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_window_counts: Option<VcetRequestedWindowCounts>,
    pub ordering: VcetTimestampOrderingEvidence,
    pub gaps: VcetGapEvidence,
}

/// Optional inputs for [`profile_vcet_dataset_coverage`].
///
/// `session_interval_mask`, when present, must contain one entry for each
/// adjacent timestamp pair. Entry `i` controls whether the interval
/// `timestamps_ms[i]..timestamps_ms[i + 1]` participates in cadence and gap
/// analysis. It does not suppress global timestamp-order validation.
#[derive(Clone, Copy, Debug, Default)]
pub struct VcetDatasetCoverageOptions<'a> {
    pub requested_bounds: Option<VcetRequestedCoverageBoundsMs>,
    pub expected_cadence_ms: Option<i64>,
    pub session_interval_mask: Option<&'a [bool]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VcetDatasetCoverageError {
    TooManyTimestamps {
        count: usize,
        max: usize,
    },
    TimestampOutOfRange {
        index: usize,
        value: i64,
        min: i64,
        max: i64,
    },
    InvalidRequestedBounds {
        start_ms: i64,
        end_exclusive_ms: i64,
    },
    InvalidExpectedCadence {
        value_ms: i64,
        max_ms: i64,
    },
    SessionMaskLengthMismatch {
        expected: usize,
        actual: usize,
    },
    ArithmeticOverflow {
        operation: &'static str,
    },
}

impl fmt::Display for VcetDatasetCoverageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyTimestamps { count, max } => write!(
                f,
                "VCET dataset coverage received {count} timestamps; maximum is {max}"
            ),
            Self::TimestampOutOfRange {
                index,
                value,
                min,
                max,
            } => write!(
                f,
                "VCET dataset coverage timestamp at index {index} is {value}; supported millisecond range is [{min}, {max}]"
            ),
            Self::InvalidRequestedBounds {
                start_ms,
                end_exclusive_ms,
            } => write!(
                f,
                "VCET requested coverage bounds must satisfy 0 <= start_ms < end_exclusive_ms <= {VCET_DATASET_COVERAGE_MAX_END_EXCLUSIVE_MS}; got [{start_ms}, {end_exclusive_ms})"
            ),
            Self::InvalidExpectedCadence { value_ms, max_ms } => write!(
                f,
                "VCET expected cadence must be within [1, {max_ms}] milliseconds; got {value_ms}"
            ),
            Self::SessionMaskLengthMismatch { expected, actual } => write!(
                f,
                "VCET session interval mask length mismatch: expected {expected}, got {actual}"
            ),
            Self::ArithmeticOverflow { operation } => {
                write!(f, "VCET dataset coverage arithmetic overflow while {operation}")
            }
        }
    }
}

impl std::error::Error for VcetDatasetCoverageError {}

/// Builds deterministic, strategy-local evidence about a VCET dataset's
/// chronological timestamp coverage.
///
/// Duplicate and descending timestamps are returned as ordering evidence, but
/// cadence and gap statistics are withheld in that case. The function never
/// sorts or repairs timestamps because doing so would conceal source-data
/// defects from downstream admission logic.
pub fn profile_vcet_dataset_coverage(
    timestamps_ms: &[i64],
    options: VcetDatasetCoverageOptions<'_>,
) -> Result<VcetDatasetCoverageEvidence, VcetDatasetCoverageError> {
    validate_timestamp_count_with_limit(timestamps_ms.len(), VCET_DATASET_COVERAGE_MAX_TIMESTAMPS)?;
    validate_options(timestamps_ms.len(), &options)?;

    let mut observed_min = None::<i64>;
    let mut observed_max = None::<i64>;
    let mut before_start = 0usize;
    let mut within_requested = 0usize;
    let mut at_or_after_end = 0usize;

    for (index, timestamp) in timestamps_ms.iter().copied().enumerate() {
        if !(0..=VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS).contains(&timestamp) {
            return Err(VcetDatasetCoverageError::TimestampOutOfRange {
                index,
                value: timestamp,
                min: 0,
                max: VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS,
            });
        }
        observed_min = Some(observed_min.map_or(timestamp, |value| value.min(timestamp)));
        observed_max = Some(observed_max.map_or(timestamp, |value| value.max(timestamp)));

        if let Some(bounds) = options.requested_bounds {
            if timestamp < bounds.start_ms {
                before_start =
                    checked_increment(before_start, "counting timestamps before bounds")?;
            } else if timestamp >= bounds.end_exclusive_ms {
                at_or_after_end =
                    checked_increment(at_or_after_end, "counting timestamps after bounds")?;
            } else {
                within_requested =
                    checked_increment(within_requested, "counting timestamps within bounds")?;
            }
        }
    }

    let observed_bounds = match (observed_min, observed_max) {
        (Some(min_ms), Some(max_ms)) => Some(VcetObservedCoverageBoundsMs {
            min_ms,
            max_ms,
            span_ms: max_ms.checked_sub(min_ms).ok_or(
                VcetDatasetCoverageError::ArithmeticOverflow {
                    operation: "computing observed timestamp span",
                },
            )?,
        }),
        _ => None,
    };

    let mut duplicate_interval_count = 0usize;
    let mut descending_interval_count = 0usize;
    let mut first_duplicate_at_index = None;
    let mut first_descending_at_index = None;
    for index in 1..timestamps_ms.len() {
        match timestamps_ms[index].cmp(&timestamps_ms[index - 1]) {
            std::cmp::Ordering::Equal => {
                duplicate_interval_count = checked_increment(
                    duplicate_interval_count,
                    "counting duplicate timestamp intervals",
                )?;
                first_duplicate_at_index.get_or_insert(index);
            }
            std::cmp::Ordering::Less => {
                descending_interval_count = checked_increment(
                    descending_interval_count,
                    "counting descending timestamp intervals",
                )?;
                first_descending_at_index.get_or_insert(index);
            }
            std::cmp::Ordering::Greater => {}
        }
    }
    let ordering = VcetTimestampOrderingEvidence {
        monotonic_non_decreasing: descending_interval_count == 0,
        strictly_increasing: duplicate_interval_count == 0 && descending_interval_count == 0,
        duplicate_interval_count,
        descending_interval_count,
        first_duplicate_at_index,
        first_descending_at_index,
    };

    let requested_window_counts = options.requested_bounds.map(|_| VcetRequestedWindowCounts {
        before_start,
        within: within_requested,
        at_or_after_end,
    });
    let gaps = build_gap_evidence(timestamps_ms, &options, ordering.strictly_increasing)?;

    Ok(VcetDatasetCoverageEvidence {
        schema_version: VCET_DATASET_COVERAGE_SCHEMA_VERSION,
        timestamp_count: timestamps_ms.len(),
        requested_bounds: options.requested_bounds,
        observed_bounds,
        requested_window_counts,
        ordering,
        gaps,
    })
}

fn validate_options(
    timestamp_count: usize,
    options: &VcetDatasetCoverageOptions<'_>,
) -> Result<(), VcetDatasetCoverageError> {
    if let Some(bounds) = options.requested_bounds {
        if bounds.start_ms < 0
            || bounds.end_exclusive_ms <= bounds.start_ms
            || bounds.end_exclusive_ms > VCET_DATASET_COVERAGE_MAX_END_EXCLUSIVE_MS
        {
            return Err(VcetDatasetCoverageError::InvalidRequestedBounds {
                start_ms: bounds.start_ms,
                end_exclusive_ms: bounds.end_exclusive_ms,
            });
        }
    }
    if let Some(value_ms) = options.expected_cadence_ms {
        if value_ms <= 0 || value_ms > VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS {
            return Err(VcetDatasetCoverageError::InvalidExpectedCadence {
                value_ms,
                max_ms: VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS,
            });
        }
    }
    if let Some(mask) = options.session_interval_mask {
        let expected = timestamp_count.saturating_sub(1);
        if mask.len() != expected {
            return Err(VcetDatasetCoverageError::SessionMaskLengthMismatch {
                expected,
                actual: mask.len(),
            });
        }
    }
    Ok(())
}

fn build_gap_evidence(
    timestamps_ms: &[i64],
    options: &VcetDatasetCoverageOptions<'_>,
    strictly_increasing: bool,
) -> Result<VcetGapEvidence, VcetDatasetCoverageError> {
    let adjacent_interval_count = timestamps_ms.len().saturating_sub(1);
    let analyzed_interval_count = options
        .session_interval_mask
        .map(|mask| mask.iter().filter(|enabled| **enabled).count())
        .unwrap_or(adjacent_interval_count);
    let excluded_interval_count = adjacent_interval_count
        .checked_sub(analyzed_interval_count)
        .ok_or(VcetDatasetCoverageError::ArithmeticOverflow {
            operation: "computing excluded interval count",
        })?;
    let basis = if options.session_interval_mask.is_some() {
        VcetGapAnalysisBasis::SessionMaskedAdjacentTimestamps
    } else {
        VcetGapAnalysisBasis::RawAdjacentTimestamps
    };
    let caveat = if options.session_interval_mask.is_some() {
        VCET_SESSION_MASK_GAP_CAVEAT
    } else {
        VCET_RAW_GAP_CAVEAT
    }
    .to_string();

    let incomplete = |status| VcetGapEvidence {
        status,
        basis,
        expected_cadence_ms: options.expected_cadence_ms,
        cadence_estimate_ms: None,
        cadence_sample_interval_count: 0,
        cadence_used_ms: None,
        cadence_source: None,
        adjacent_interval_count,
        analyzed_interval_count,
        excluded_interval_count,
        irregular_interval_count: None,
        gap_interval_count: None,
        missing_interval_count: None,
        longest_gap_ms: None,
        longest_gap_missing_interval_count: None,
        caveat: caveat.clone(),
    };

    if timestamps_ms.len() < 2 {
        return Ok(incomplete(VcetGapAnalysisStatus::InsufficientObservations));
    }
    if !strictly_increasing {
        return Ok(incomplete(VcetGapAnalysisStatus::InvalidTimestampOrder));
    }
    if analyzed_interval_count == 0 {
        return Ok(incomplete(VcetGapAnalysisStatus::NoEligibleIntervals));
    }

    let mut cadence_sample = collect_uniform_delta_sample(
        timestamps_ms,
        options.session_interval_mask,
        analyzed_interval_count,
        VCET_DATASET_COVERAGE_MAX_CADENCE_SAMPLE,
    )?;
    cadence_sample.sort_unstable();
    let cadence_estimate_ms = cadence_sample[cadence_sample.len() / 2];
    let (cadence_used_ms, cadence_source) = match options.expected_cadence_ms {
        Some(expected) => (expected, VcetGapCadenceSource::SuppliedExpectedCadence),
        None => (
            cadence_estimate_ms,
            VcetGapCadenceSource::EstimatedUpperMedianPositiveDelta,
        ),
    };

    let mut irregular_interval_count = 0usize;
    let mut gap_interval_count = 0usize;
    let mut missing_interval_count = 0u64;
    let mut longest_gap_ms = None::<i64>;
    let mut longest_gap_missing_interval_count = None::<u64>;

    for index in 0..adjacent_interval_count {
        if !interval_enabled(options.session_interval_mask, index) {
            continue;
        }
        let delta = timestamps_ms[index + 1]
            .checked_sub(timestamps_ms[index])
            .ok_or(VcetDatasetCoverageError::ArithmeticOverflow {
                operation: "computing an adjacent timestamp delta",
            })?;
        if delta % cadence_used_ms != 0 {
            irregular_interval_count = checked_increment(
                irregular_interval_count,
                "counting cadence-irregular intervals",
            )?;
        }
        let missing = (delta / cadence_used_ms).saturating_sub(1) as u64;
        if missing == 0 {
            continue;
        }
        gap_interval_count =
            checked_increment(gap_interval_count, "counting timestamp gap intervals")?;
        missing_interval_count = missing_interval_count.checked_add(missing).ok_or(
            VcetDatasetCoverageError::ArithmeticOverflow {
                operation: "summing missing timestamp intervals",
            },
        )?;
        if longest_gap_ms.is_none_or(|longest| delta > longest) {
            longest_gap_ms = Some(delta);
            longest_gap_missing_interval_count = Some(missing);
        }
    }

    Ok(VcetGapEvidence {
        status: VcetGapAnalysisStatus::Complete,
        basis,
        expected_cadence_ms: options.expected_cadence_ms,
        cadence_estimate_ms: Some(cadence_estimate_ms),
        cadence_sample_interval_count: cadence_sample.len(),
        cadence_used_ms: Some(cadence_used_ms),
        cadence_source: Some(cadence_source),
        adjacent_interval_count,
        analyzed_interval_count,
        excluded_interval_count,
        irregular_interval_count: Some(irregular_interval_count),
        gap_interval_count: Some(gap_interval_count),
        missing_interval_count: Some(missing_interval_count),
        longest_gap_ms,
        longest_gap_missing_interval_count,
        caveat,
    })
}

fn collect_uniform_delta_sample(
    timestamps_ms: &[i64],
    session_interval_mask: Option<&[bool]>,
    eligible_interval_count: usize,
    sample_cap: usize,
) -> Result<Vec<i64>, VcetDatasetCoverageError> {
    if sample_cap == 0 {
        return Err(VcetDatasetCoverageError::ArithmeticOverflow {
            operation: "building a zero-cap cadence sample",
        });
    }
    let target_count = eligible_interval_count.min(sample_cap);
    let mut sample = Vec::with_capacity(target_count);
    let mut eligible_rank = 0usize;
    let mut target_index = 0usize;

    for index in 0..timestamps_ms.len().saturating_sub(1) {
        if !interval_enabled(session_interval_mask, index) {
            continue;
        }
        let target_rank = uniform_sample_rank(target_index, target_count, eligible_interval_count);
        if eligible_rank == target_rank {
            let delta = timestamps_ms[index + 1]
                .checked_sub(timestamps_ms[index])
                .ok_or(VcetDatasetCoverageError::ArithmeticOverflow {
                    operation: "sampling an adjacent timestamp delta",
                })?;
            sample.push(delta);
            target_index += 1;
            if target_index == target_count {
                break;
            }
        }
        eligible_rank += 1;
    }

    if sample.len() != target_count {
        return Err(VcetDatasetCoverageError::ArithmeticOverflow {
            operation: "building the deterministic cadence sample",
        });
    }
    Ok(sample)
}

fn uniform_sample_rank(index: usize, sample_count: usize, population_count: usize) -> usize {
    debug_assert!(sample_count > 0);
    debug_assert!(population_count >= sample_count);
    if sample_count == 1 {
        return population_count / 2;
    }
    (((index as u128) * ((population_count - 1) as u128)) / ((sample_count - 1) as u128)) as usize
}

fn interval_enabled(mask: Option<&[bool]>, index: usize) -> bool {
    mask.is_none_or(|values| values[index])
}

fn checked_increment(
    value: usize,
    operation: &'static str,
) -> Result<usize, VcetDatasetCoverageError> {
    value
        .checked_add(1)
        .ok_or(VcetDatasetCoverageError::ArithmeticOverflow { operation })
}

fn validate_timestamp_count_with_limit(
    count: usize,
    max: usize,
) -> Result<(), VcetDatasetCoverageError> {
    if count > max {
        return Err(VcetDatasetCoverageError::TooManyTimestamps { count, max });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(
        timestamps_ms: &[i64],
    ) -> Result<VcetDatasetCoverageEvidence, VcetDatasetCoverageError> {
        profile_vcet_dataset_coverage(timestamps_ms, VcetDatasetCoverageOptions::default())
    }

    #[test]
    fn regular_raw_series_reports_complete_zero_gap_evidence() {
        let evidence = profile(&[0, 60_000, 120_000, 180_000]).expect("coverage evidence");

        assert_eq!(evidence.schema_version, 1);
        assert_eq!(evidence.timestamp_count, 4);
        assert_eq!(
            evidence.observed_bounds,
            Some(VcetObservedCoverageBoundsMs {
                min_ms: 0,
                max_ms: 180_000,
                span_ms: 180_000,
            })
        );
        assert!(evidence.ordering.strictly_increasing);
        assert_eq!(evidence.gaps.status, VcetGapAnalysisStatus::Complete);
        assert_eq!(
            evidence.gaps.basis,
            VcetGapAnalysisBasis::RawAdjacentTimestamps
        );
        assert_eq!(evidence.gaps.cadence_estimate_ms, Some(60_000));
        assert_eq!(evidence.gaps.cadence_used_ms, Some(60_000));
        assert_eq!(evidence.gaps.gap_interval_count, Some(0));
        assert_eq!(evidence.gaps.missing_interval_count, Some(0));
        assert_eq!(evidence.gaps.longest_gap_ms, None);
        assert_eq!(evidence.gaps.caveat, VCET_RAW_GAP_CAVEAT);
    }

    #[test]
    fn raw_series_counts_missing_intervals_and_longest_gap() {
        let evidence = profile(&[0, 60, 120, 300, 360, 720]).expect("coverage evidence");

        assert_eq!(evidence.gaps.cadence_estimate_ms, Some(60));
        assert_eq!(evidence.gaps.gap_interval_count, Some(2));
        assert_eq!(evidence.gaps.missing_interval_count, Some(7));
        assert_eq!(evidence.gaps.longest_gap_ms, Some(360));
        assert_eq!(evidence.gaps.longest_gap_missing_interval_count, Some(5));
    }

    #[test]
    fn session_mask_excludes_expected_market_closure_intervals() {
        let timestamps = [0, 60, 120, 86_400, 86_460];
        let mask = [true, true, false, true];
        let evidence = profile_vcet_dataset_coverage(
            &timestamps,
            VcetDatasetCoverageOptions {
                session_interval_mask: Some(&mask),
                ..Default::default()
            },
        )
        .expect("session-aware coverage evidence");

        assert_eq!(
            evidence.gaps.basis,
            VcetGapAnalysisBasis::SessionMaskedAdjacentTimestamps
        );
        assert_eq!(evidence.gaps.analyzed_interval_count, 3);
        assert_eq!(evidence.gaps.excluded_interval_count, 1);
        assert_eq!(evidence.gaps.cadence_estimate_ms, Some(60));
        assert_eq!(evidence.gaps.gap_interval_count, Some(0));
        assert_eq!(evidence.gaps.caveat, VCET_SESSION_MASK_GAP_CAVEAT);
    }

    #[test]
    fn requested_bounds_are_retained_and_partition_observations() {
        let requested = VcetRequestedCoverageBoundsMs {
            start_ms: 100,
            end_exclusive_ms: 250,
        };
        let evidence = profile_vcet_dataset_coverage(
            &[50, 100, 150, 200, 250],
            VcetDatasetCoverageOptions {
                requested_bounds: Some(requested),
                expected_cadence_ms: Some(50),
                session_interval_mask: None,
            },
        )
        .expect("bounded coverage evidence");

        assert_eq!(evidence.requested_bounds, Some(requested));
        assert_eq!(
            evidence.requested_window_counts,
            Some(VcetRequestedWindowCounts {
                before_start: 1,
                within: 3,
                at_or_after_end: 1,
            })
        );
        assert_eq!(
            evidence.gaps.cadence_source,
            Some(VcetGapCadenceSource::SuppliedExpectedCadence)
        );
    }

    #[test]
    fn duplicates_and_descents_are_reported_without_gap_claims() {
        let evidence = profile(&[0, 60, 60, 30, 90]).expect("ordering evidence");

        assert!(!evidence.ordering.monotonic_non_decreasing);
        assert!(!evidence.ordering.strictly_increasing);
        assert_eq!(evidence.ordering.duplicate_interval_count, 1);
        assert_eq!(evidence.ordering.descending_interval_count, 1);
        assert_eq!(evidence.ordering.first_duplicate_at_index, Some(2));
        assert_eq!(evidence.ordering.first_descending_at_index, Some(3));
        assert_eq!(
            evidence.gaps.status,
            VcetGapAnalysisStatus::InvalidTimestampOrder
        );
        assert_eq!(evidence.gaps.cadence_estimate_ms, None);
        assert_eq!(evidence.gaps.missing_interval_count, None);
    }

    #[test]
    fn duplicates_without_descents_are_non_decreasing_but_not_strict() {
        let evidence = profile(&[10, 10, 20]).expect("ordering evidence");

        assert!(evidence.ordering.monotonic_non_decreasing);
        assert!(!evidence.ordering.strictly_increasing);
        assert_eq!(evidence.ordering.duplicate_interval_count, 1);
        assert_eq!(evidence.ordering.descending_interval_count, 0);
    }

    #[test]
    fn empty_and_single_observation_series_are_explicitly_insufficient() {
        let empty = profile(&[]).expect("empty evidence");
        assert_eq!(empty.observed_bounds, None);
        assert_eq!(
            empty.gaps.status,
            VcetGapAnalysisStatus::InsufficientObservations
        );

        let single = profile(&[42]).expect("single evidence");
        assert_eq!(
            single.observed_bounds,
            Some(VcetObservedCoverageBoundsMs {
                min_ms: 42,
                max_ms: 42,
                span_ms: 0,
            })
        );
        assert_eq!(
            single.gaps.status,
            VcetGapAnalysisStatus::InsufficientObservations
        );
    }

    #[test]
    fn all_false_session_mask_withholds_cadence_and_gap_statistics() {
        let mask = [false, false];
        let evidence = profile_vcet_dataset_coverage(
            &[0, 60, 120],
            VcetDatasetCoverageOptions {
                session_interval_mask: Some(&mask),
                ..Default::default()
            },
        )
        .expect("masked evidence");

        assert_eq!(
            evidence.gaps.status,
            VcetGapAnalysisStatus::NoEligibleIntervals
        );
        assert_eq!(evidence.gaps.analyzed_interval_count, 0);
        assert_eq!(evidence.gaps.excluded_interval_count, 2);
        assert_eq!(evidence.gaps.cadence_estimate_ms, None);
    }

    #[test]
    fn cadence_irregularity_is_separate_from_full_missing_intervals() {
        let evidence = profile(&[0, 60, 121, 181]).expect("coverage evidence");

        assert_eq!(evidence.gaps.cadence_estimate_ms, Some(60));
        assert_eq!(evidence.gaps.irregular_interval_count, Some(1));
        assert_eq!(evidence.gaps.gap_interval_count, Some(0));
        assert_eq!(evidence.gaps.missing_interval_count, Some(0));
    }

    #[test]
    fn supplied_cadence_controls_missing_interval_accounting() {
        let evidence = profile_vcet_dataset_coverage(
            &[0, 60, 120],
            VcetDatasetCoverageOptions {
                expected_cadence_ms: Some(30),
                ..Default::default()
            },
        )
        .expect("supplied cadence evidence");

        assert_eq!(evidence.gaps.cadence_estimate_ms, Some(60));
        assert_eq!(evidence.gaps.cadence_used_ms, Some(30));
        assert_eq!(evidence.gaps.gap_interval_count, Some(2));
        assert_eq!(evidence.gaps.missing_interval_count, Some(2));
    }

    #[test]
    fn deterministic_sampling_spans_the_delta_population() {
        let timestamps = [0, 10, 30, 60, 100, 150];
        let sample = collect_uniform_delta_sample(&timestamps, None, 5, 3)
            .expect("deterministic cadence sample");

        assert_eq!(sample, vec![10, 30, 50]);
        assert_eq!(uniform_sample_rank(0, 3, 5), 0);
        assert_eq!(uniform_sample_rank(1, 3, 5), 2);
        assert_eq!(uniform_sample_rank(2, 3, 5), 4);
        assert_eq!(uniform_sample_rank(0, 1, 5), 2);
    }

    #[test]
    fn session_mask_length_must_match_adjacent_intervals() {
        let mask = [true];
        let error = profile_vcet_dataset_coverage(
            &[0, 60, 120],
            VcetDatasetCoverageOptions {
                session_interval_mask: Some(&mask),
                ..Default::default()
            },
        )
        .expect_err("mask length mismatch");

        assert_eq!(
            error,
            VcetDatasetCoverageError::SessionMaskLengthMismatch {
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn invalid_requested_bounds_fail_closed() {
        for requested_bounds in [
            VcetRequestedCoverageBoundsMs {
                start_ms: -1,
                end_exclusive_ms: 10,
            },
            VcetRequestedCoverageBoundsMs {
                start_ms: 10,
                end_exclusive_ms: 10,
            },
            VcetRequestedCoverageBoundsMs {
                start_ms: 11,
                end_exclusive_ms: 10,
            },
            VcetRequestedCoverageBoundsMs {
                start_ms: 0,
                end_exclusive_ms: VCET_DATASET_COVERAGE_MAX_END_EXCLUSIVE_MS + 1,
            },
        ] {
            let error = profile_vcet_dataset_coverage(
                &[0, 1],
                VcetDatasetCoverageOptions {
                    requested_bounds: Some(requested_bounds),
                    ..Default::default()
                },
            )
            .expect_err("invalid bounds");
            assert!(matches!(
                error,
                VcetDatasetCoverageError::InvalidRequestedBounds { .. }
            ));
        }
    }

    #[test]
    fn requested_end_exclusive_can_include_the_maximum_supported_timestamp() {
        let max = VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS;
        let evidence = profile_vcet_dataset_coverage(
            &[max - 1, max],
            VcetDatasetCoverageOptions {
                requested_bounds: Some(VcetRequestedCoverageBoundsMs {
                    start_ms: max,
                    end_exclusive_ms: VCET_DATASET_COVERAGE_MAX_END_EXCLUSIVE_MS,
                }),
                ..Default::default()
            },
        )
        .expect("maximum timestamp bounds");

        assert_eq!(
            evidence.requested_window_counts,
            Some(VcetRequestedWindowCounts {
                before_start: 1,
                within: 1,
                at_or_after_end: 0,
            })
        );
    }

    #[test]
    fn invalid_timestamp_and_cadence_values_fail_closed() {
        for value in [-1, VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS + 1, i64::MAX] {
            let error = profile(&[0, value]).expect_err("invalid timestamp");
            assert!(matches!(
                error,
                VcetDatasetCoverageError::TimestampOutOfRange { index: 1, .. }
            ));
        }

        for value_ms in [0, -1, VCET_DATASET_COVERAGE_MAX_TIMESTAMP_MS + 1] {
            let error = profile_vcet_dataset_coverage(
                &[0, 1],
                VcetDatasetCoverageOptions {
                    expected_cadence_ms: Some(value_ms),
                    ..Default::default()
                },
            )
            .expect_err("invalid cadence");
            assert!(matches!(
                error,
                VcetDatasetCoverageError::InvalidExpectedCadence { .. }
            ));
        }
    }

    #[test]
    fn timestamp_count_bound_fails_closed_without_large_allocation() {
        let error = validate_timestamp_count_with_limit(4, 3).expect_err("count bound");
        assert_eq!(
            error,
            VcetDatasetCoverageError::TooManyTimestamps { count: 4, max: 3 }
        );
    }

    #[test]
    fn evidence_round_trips_through_json_with_explicit_caveat() {
        let evidence = profile(&[0, 60, 120]).expect("coverage evidence");
        let json = serde_json::to_string(&evidence).expect("serialize evidence");
        assert!(json.contains("raw_adjacent_timestamps"));
        assert!(json.contains("market closures"));

        let round_trip: VcetDatasetCoverageEvidence =
            serde_json::from_str(&json).expect("deserialize evidence");
        assert_eq!(round_trip, evidence);
    }
}
